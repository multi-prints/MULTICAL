import { Hono } from "hono";
import { newDistributedId, turso } from "../db";
import type { Env } from "../env";

type AppEnv = { Bindings: Env };

export const debts = new Hono<AppEnv>();

function mapDebt(row: Record<string, unknown>) {
  return {
    ...row,
    source_label: row.source_label ?? row.description ?? null,
    source_kind:
      row.source_kind ??
      (row.sale_id
        ? "sale"
        : row.service_transaction_id
          ? "printing"
          : "manual"),
    source_detail: row.source_detail ?? null,
    source_sale_type: row.source_sale_type ?? null,
    source_product_type: row.source_product_type ?? null,
    source_color: row.source_color ?? null,
    source_sticker_type: row.source_sticker_type ?? null,
    last_payment_at: row.last_payment_at ?? null,
  };
}

const debtSelect = `SELECT *,
  (SELECT MAX(payment_date) FROM debt_payments dp WHERE dp.debt_id = debts.id) AS last_payment_at
 FROM debts`;

debts.get("/", async (c) => {
  const db = turso(c.env);
  const page = Math.max(1, Number(c.req.query("page") ?? 1));
  const perPage = Math.min(200, Math.max(1, Number(c.req.query("per_page") ?? 50)));
  const offset = (page - 1) * perPage;
  const search = (c.req.query("search") ?? "").trim().toLowerCase();
  const sortBy = c.req.query("sort_by") ?? "newest";

  const order =
    sortBy === "oldest"
      ? "created_at ASC"
      : sortBy === "amount_desc"
        ? "remaining_amount DESC, created_at DESC"
        : sortBy === "amount_asc"
          ? "remaining_amount ASC, created_at DESC"
          : "created_at DESC";

  let where = "";
  const args: (string | number)[] = [];
  if (search) {
    where = `WHERE (
      LOWER(customer_name) LIKE ? OR
      LOWER(COALESCE(phone,'')) LIKE ? OR
      LOWER(COALESCE(description,'')) LIKE ? OR
      LOWER(status) LIKE ?
    )`;
    const q = `%${search}%`;
    args.push(q, q, q, q);
  }

  const count = await db.execute({
    sql: `SELECT COUNT(*) AS n FROM debts ${where}`,
    args,
  });
  const total = Number(count.rows[0]?.n ?? 0);

  const list = await db.execute({
    sql: `${debtSelect} ${where} ORDER BY ${order} LIMIT ? OFFSET ?`,
    args: [...args, perPage, offset],
  });

  const metrics = await db.execute(`
    SELECT
      COALESCE(SUM(CASE WHEN status = 'pending' THEN remaining_amount ELSE 0 END), 0) AS total_outstanding,
      COALESCE((
        SELECT SUM(amount) FROM debt_payments
        WHERE strftime('%Y-%m', payment_date) = strftime('%Y-%m', 'now', 'localtime')
      ), 0) AS paid_this_month,
      COALESCE(SUM(CASE WHEN status = 'pending' AND due_date IS NOT NULL
        AND date(due_date) < date('now', 'localtime') THEN 1 ELSE 0 END), 0) AS overdue_count
    FROM debts
  `);
  const m = metrics.rows[0] ?? {};

  const all = await db.execute(
    `${debtSelect} ORDER BY created_at DESC LIMIT 500`,
  );

  return c.json({
    items: list.rows.map((r) => mapDebt(r as Record<string, unknown>)),
    total_count: total,
    page,
    per_page: perPage,
    total_outstanding: Number(m.total_outstanding ?? 0),
    paid_this_month: Number(m.paid_this_month ?? 0),
    overdue_count: Number(m.overdue_count ?? 0),
    all_debts: all.rows.map((r) => mapDebt(r as Record<string, unknown>)),
  });
});

debts.get("/pending", async (c) => {
  const db = turso(c.env);
  const res = await db.execute(
    `${debtSelect} WHERE status = 'pending' ORDER BY created_at DESC`,
  );
  return c.json({
    items: res.rows.map((r) => mapDebt(r as Record<string, unknown>)),
  });
});

debts.get("/overdue", async (c) => {
  const db = turso(c.env);
  const res = await db.execute(
    `${debtSelect}
     WHERE status = 'pending' AND due_date IS NOT NULL
       AND date(due_date) < date('now', 'localtime')
     ORDER BY due_date ASC`,
  );
  return c.json({
    items: res.rows.map((r) => mapDebt(r as Record<string, unknown>)),
  });
});

debts.get("/metrics", async (c) => {
  const db = turso(c.env);
  const outstanding = await db.execute(
    `SELECT COALESCE(SUM(remaining_amount), 0) AS v FROM debts WHERE status = 'pending'`,
  );
  const paid = await db.execute(
    `SELECT COALESCE(SUM(amount), 0) AS v FROM debt_payments
     WHERE strftime('%Y-%m', payment_date) = strftime('%Y-%m', 'now', 'localtime')`,
  );
  return c.json({
    total_outstanding: Number(outstanding.rows[0]?.v ?? 0),
    paid_this_month: Number(paid.rows[0]?.v ?? 0),
  });
});

debts.get("/by-sale/:saleId", async (c) => {
  const saleId = Number(c.req.param("saleId"));
  const db = turso(c.env);
  const res = await db.execute({
    sql: `${debtSelect} WHERE sale_id = ? ORDER BY created_at DESC LIMIT 1`,
    args: [saleId],
  });
  if (!res.rows.length) return c.json(null);
  return c.json(mapDebt(res.rows[0] as Record<string, unknown>));
});

debts.get("/by-transaction/:txnId", async (c) => {
  const txnId = Number(c.req.param("txnId"));
  const db = turso(c.env);
  const res = await db.execute({
    sql: `${debtSelect} WHERE service_transaction_id = ? ORDER BY created_at DESC LIMIT 1`,
    args: [txnId],
  });
  if (!res.rows.length) return c.json(null);
  return c.json(mapDebt(res.rows[0] as Record<string, unknown>));
});

debts.post("/", async (c) => {
  const body = await c.req.json<{
    customer_name: string;
    phone?: string | null;
    amount: number;
    paid_amount?: number | null;
    remaining_amount?: number | null;
    due_date?: string | null;
    description?: string | null;
    sale_id?: number | null;
    service_transaction_id?: number | null;
  }>();

  if (!body.customer_name?.trim() || body.amount == null) {
    return c.json({ error: "customer_name and amount required" }, 400);
  }

  const paid = Number(body.paid_amount ?? 0);
  const remaining = Number(
    body.remaining_amount ?? Math.max(0, body.amount - paid),
  );
  const status = remaining <= 0 ? "paid" : "pending";
  const now = new Date().toISOString();
  const paidAt = status === "paid" ? now : null;
  const id = newDistributedId();
  const db = turso(c.env);

  await db.execute({
    sql: `INSERT INTO debts
            (id, customer_name, phone, amount, paid_amount, remaining_amount,
             due_date, description, status, sale_id, service_transaction_id, paid_at, created_at)
          VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
    args: [
      id,
      body.customer_name.trim(),
      body.phone ?? null,
      body.amount,
      paid,
      remaining,
      body.due_date ?? null,
      body.description ?? null,
      status,
      body.sale_id ?? null,
      body.service_transaction_id ?? null,
      paidAt,
      now,
    ],
  });

  if (paid > 0) {
    await db.execute({
      sql: `INSERT INTO debt_payments (id, debt_id, amount, payment_method, notes, payment_date)
            VALUES (?, ?, ?, 'cash', 'Initial payment', ?)`,
      args: [newDistributedId(), id, paid, now],
    });
  }

  const row = await db.execute({
    sql: `${debtSelect} WHERE id = ?`,
    args: [id],
  });
  return c.json(mapDebt(row.rows[0] as Record<string, unknown>), 201);
});

debts.post("/:id/mark-paid", async (c) => {
  const id = Number(c.req.param("id"));
  const db = turso(c.env);
  const cur = await db.execute({
    sql: "SELECT amount, remaining_amount FROM debts WHERE id = ?",
    args: [id],
  });
  if (!cur.rows.length) return c.json({ error: "Not found" }, 404);
  const amount = Number(cur.rows[0].amount ?? 0);
  const rem = Number(cur.rows[0].remaining_amount ?? 0);
  const now = new Date().toISOString();
  if (rem > 0) {
    await db.execute({
      sql: `INSERT INTO debt_payments (id, debt_id, amount, payment_method, notes, payment_date)
            VALUES (?, ?, ?, 'cash', 'Marked paid', ?)`,
      args: [newDistributedId(), id, rem, now],
    });
  }
  await db.execute({
    sql: `UPDATE debts SET paid_amount = ?, remaining_amount = 0, status = 'paid', paid_at = ?
          WHERE id = ?`,
    args: [amount, now, id],
  });
  return c.json({ success: true });
});

debts.post("/:id/payments", async (c) => {
  const id = Number(c.req.param("id"));
  const body = await c.req.json<{
    amount: number;
    payment_method?: string;
    notes?: string | null;
  }>();
  const pay = Number(body.amount);
  if (!(pay > 0)) return c.json({ error: "amount must be > 0" }, 400);

  const db = turso(c.env);
  const cur = await db.execute({
    sql: "SELECT amount, paid_amount FROM debts WHERE id = ?",
    args: [id],
  });
  if (!cur.rows.length) return c.json({ error: "Not found" }, 404);

  const paid = Number(cur.rows[0].paid_amount ?? 0) + pay;
  const amount = Number(cur.rows[0].amount ?? 0);
  const remaining = Math.max(0, amount - paid);
  const status = remaining <= 0 ? "paid" : "pending";
  const now = new Date().toISOString();
  const paymentId = newDistributedId();

  await db.execute({
    sql: `INSERT INTO debt_payments (id, debt_id, amount, payment_method, notes, payment_date)
          VALUES (?, ?, ?, ?, ?, ?)`,
    args: [
      paymentId,
      id,
      pay,
      body.payment_method ?? "cash",
      body.notes ?? null,
      now,
    ],
  });
  await db.execute({
    sql: `UPDATE debts SET paid_amount = ?, remaining_amount = ?, status = ?,
            paid_at = CASE WHEN ? = 'paid' THEN ? ELSE paid_at END
          WHERE id = ?`,
    args: [paid, remaining, status, status, now, id],
  });

  const row = await db.execute({
    sql: "SELECT * FROM debt_payments WHERE id = ?",
    args: [paymentId],
  });
  return c.json(row.rows[0], 201);
});

debts.get("/:id/payments", async (c) => {
  const id = Number(c.req.param("id"));
  const db = turso(c.env);
  const res = await db.execute({
    sql: `SELECT * FROM debt_payments WHERE debt_id = ? ORDER BY payment_date DESC`,
    args: [id],
  });
  return c.json({ items: res.rows });
});

debts.patch("/:id", async (c) => {
  const id = Number(c.req.param("id"));
  const body = await c.req.json<Record<string, unknown>>();
  const fields: string[] = [];
  const args: (string | number | null)[] = [];
  for (const k of [
    "customer_name",
    "phone",
    "amount",
    "paid_amount",
    "remaining_amount",
    "due_date",
    "description",
    "status",
  ]) {
    if (body[k] !== undefined) {
      fields.push(`${k} = ?`);
      const v = body[k];
      if (v === null) args.push(null);
      else if (typeof v === "number") args.push(v);
      else args.push(String(v));
    }
  }
  if (!fields.length) return c.json({ error: "No fields" }, 400);
  args.push(id);
  const db = turso(c.env);
  await db.execute({
    sql: `UPDATE debts SET ${fields.join(", ")} WHERE id = ?`,
    args,
  });
  return c.json({ success: true });
});

debts.delete("/:id", async (c) => {
  const id = Number(c.req.param("id"));
  const db = turso(c.env);
  await db.execute({
    sql: "DELETE FROM debt_payments WHERE debt_id = ?",
    args: [id],
  });
  await db.execute({ sql: "DELETE FROM debts WHERE id = ?", args: [id] });
  return c.json({ success: true });
});

debts.delete("/payments/:paymentId", async (c) => {
  const paymentId = Number(c.req.param("paymentId"));
  const db = turso(c.env);
  await db.execute({
    sql: "DELETE FROM debt_payments WHERE id = ?",
    args: [paymentId],
  });
  return c.json({ success: true });
});
