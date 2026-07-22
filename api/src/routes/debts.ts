import { Hono } from "hono";
import { asId, newDistributedId, turso } from "../db";
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

/**
 * Full debt row + sale/product/stock/print joins so the UI can show the same
 * previews as the desktop Tauri list (color swatches, product type, etc.).
 */
const debtSelect = `
SELECT
  d.id, d.customer_name, d.phone, d.amount, d.paid_amount, d.remaining_amount,
  d.due_date, d.description, d.status, d.sale_id, d.service_transaction_id,
  d.paid_at, d.created_at,
  (SELECT MAX(payment_date) FROM debt_payments dp WHERE dp.debt_id = d.id) AS last_payment_at,
  CASE
    WHEN d.sale_id IS NOT NULL THEN COALESCE(
      NULLIF(TRIM(d.description), ''),
      NULLIF(TRIM(s.product_name), ''),
      CASE
        WHEN s.type = 'stock' THEN TRIM(
          COALESCE(sk.color, '') || ' ' ||
          CASE WHEN COALESCE(s.sticker_type, sk.sticker_type, '') = 'reflective'
            THEN 'Reflective' ELSE 'Colored' END || ' Sticker'
        )
        WHEN s.type = 'service' THEN 'Service sale'
        WHEN s.type = 'product' THEN 'Product sale'
        ELSE 'Sale'
      END
    )
    WHEN d.service_transaction_id IS NOT NULL THEN COALESCE(
      NULLIF(TRIM(st.service_name), ''),
      NULLIF(TRIM(d.description), ''),
      'Printing job'
    )
    ELSE COALESCE(NULLIF(TRIM(d.description), ''), 'Manual debt')
  END AS source_label,
  CASE
    WHEN d.sale_id IS NOT NULL THEN 'sale'
    WHEN d.service_transaction_id IS NOT NULL THEN 'printing'
    ELSE 'manual'
  END AS source_kind,
  CASE
    WHEN d.sale_id IS NOT NULL AND s.type = 'product' THEN TRIM(
      CASE s.product_type
        WHEN 'life_saver' THEN 'Lifesaver'
        WHEN 'chevron' THEN 'Chevron'
        WHEN 'stripes' THEN 'Stripes'
        ELSE COALESCE(s.product_type, 'Product')
      END ||
      CASE WHEN COALESCE(p.color, '') != '' THEN ' · ' || CASE p.color
        WHEN 'white_red' THEN 'White / Red'
        WHEN 'yellow_red' THEN 'Yellow / Red'
        WHEN 'white' THEN 'White'
        WHEN 'yellow' THEN 'Yellow'
        ELSE p.color
      END ELSE '' END ||
      CASE WHEN COALESCE(s.quantity, '') != '' THEN ' · qty ' || s.quantity ELSE '' END
    )
    WHEN d.sale_id IS NOT NULL AND s.type = 'stock' THEN TRIM(
      CASE WHEN COALESCE(s.sticker_type, sk.sticker_type, '') = 'reflective'
        THEN 'Reflective' ELSE 'Colored' END ||
      CASE WHEN COALESCE(sk.color, '') != '' THEN ' · ' || sk.color ELSE '' END ||
      CASE WHEN COALESCE(sk.size, s.quantity, '') != ''
        THEN ' · ' || COALESCE(sk.size, s.quantity) ELSE '' END
    )
    WHEN d.sale_id IS NOT NULL THEN TRIM(COALESCE(s.quantity, ''))
    WHEN d.service_transaction_id IS NOT NULL THEN TRIM(
      CASE WHEN st.stock_metres_used > 0
        THEN printf('%.1fm printed', st.stock_metres_used) ELSE '' END ||
      CASE WHEN st.material_type IS NOT NULL AND TRIM(st.material_type) != ''
        THEN ' · ' || st.material_type ELSE '' END ||
      CASE WHEN st.material_size IS NOT NULL AND TRIM(st.material_size) != ''
        THEN ' · ' || st.material_size || 'm wide' ELSE '' END
    )
    ELSE NULL
  END AS source_detail,
  s.type AS source_sale_type,
  s.product_type AS source_product_type,
  COALESCE(p.color, sk.color) AS source_color,
  COALESCE(s.sticker_type, sk.sticker_type) AS source_sticker_type
FROM debts d
LEFT JOIN sales s ON s.id = d.sale_id
LEFT JOIN products p ON p.id = s.product_id
LEFT JOIN stock sk ON sk.id = s.stock_id
LEFT JOIN service_transactions st ON st.id = d.service_transaction_id
`;

debts.get("/", async (c) => {
  const db = turso(c.env);
  const page = Math.max(1, Number(c.req.query("page") ?? 1));
  const perPage = Math.min(200, Math.max(1, Number(c.req.query("per_page") ?? 50)));
  const offset = (page - 1) * perPage;
  const search = (c.req.query("search") ?? "").trim().toLowerCase();
  const sortBy = c.req.query("sort_by") ?? "newest";

  const order =
    sortBy === "oldest"
      ? "d.created_at ASC"
      : sortBy === "amount_desc"
        ? "d.remaining_amount DESC, d.created_at DESC"
        : sortBy === "amount_asc"
          ? "d.remaining_amount ASC, d.created_at DESC"
          : "d.created_at DESC";

  let where = "";
  const args: (string | number)[] = [];
  if (search) {
    where = `WHERE (
      LOWER(d.customer_name) LIKE ? OR
      LOWER(COALESCE(d.phone,'')) LIKE ? OR
      LOWER(COALESCE(d.description,'')) LIKE ? OR
      LOWER(d.status) LIKE ?
    )`;
    const q = `%${search}%`;
    args.push(q, q, q, q);
  }

  const count = await db.execute({
    sql: `SELECT COUNT(*) AS n FROM debts d ${where}`,
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
    `${debtSelect} ORDER BY d.created_at DESC LIMIT 500`,
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
    `${debtSelect} WHERE d.status = 'pending' ORDER BY d.created_at DESC`,
  );
  return c.json({
    items: res.rows.map((r) => mapDebt(r as Record<string, unknown>)),
  });
});

debts.get("/overdue", async (c) => {
  const db = turso(c.env);
  const res = await db.execute(
    `${debtSelect}
     WHERE d.status = 'pending' AND d.due_date IS NOT NULL
       AND date(d.due_date) < date('now', 'localtime')
     ORDER BY d.due_date ASC`,
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
    sql: `${debtSelect} WHERE d.sale_id = ? ORDER BY d.created_at DESC LIMIT 1`,
    args: [saleId],
  });
  if (!res.rows.length) return c.json(null);
  return c.json(mapDebt(res.rows[0] as Record<string, unknown>));
});

debts.get("/by-transaction/:txnId", async (c) => {
  const txnId = Number(c.req.param("txnId"));
  const db = turso(c.env);
  const res = await db.execute({
    sql: `${debtSelect} WHERE d.service_transaction_id = ? ORDER BY d.created_at DESC LIMIT 1`,
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
    sale_id?: number | string | null;
    service_transaction_id?: number | string | null;
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
  const saleId = asId(body.sale_id);
  const txnId = asId(body.service_transaction_id);
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
      saleId,
      txnId,
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

  // Mark linked sale / printing job as debt in the same request so the client
  // shows the Debt tag without a second local Tauri update (Windows hang).
  if (saleId != null) {
    const upd = await db.execute({
      sql: `UPDATE sales SET is_debt = 1 WHERE id = ?`,
      args: [saleId],
    });
    if ((upd.rowsAffected ?? 0) === 0) {
      console.warn(`debts.post: no sale row for is_debt mark id=${saleId}`);
    }
  }
  if (txnId != null) {
    const upd = await db.execute({
      sql: `UPDATE service_transactions SET is_debt = 1 WHERE id = ?`,
      args: [txnId],
    });
    if ((upd.rowsAffected ?? 0) === 0) {
      console.warn(
        `debts.post: no service_transaction row for is_debt mark id=${txnId}`,
      );
    }
  }

  const row = await db.execute({
    sql: `${debtSelect} WHERE d.id = ?`,
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