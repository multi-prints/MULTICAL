import { Hono } from "hono";
import { asId, asInt, newDistributedId, turso } from "../db";
import type { Env } from "../env";

type AppEnv = { Bindings: Env };

export const printing = new Hono<AppEnv>();

function mapJob(row: Record<string, unknown>) {
  const isDebt = asInt(row.is_debt, 0);
  const amount = Number(row.amount ?? 0);
  const amountPaid =
    row.amount_paid != null
      ? Number(row.amount_paid)
      : isDebt === 0
        ? amount
        : Number(row.debt_paid ?? 0);
  return {
    ...row,
    id: row.id != null ? String(row.id) : row.id,
    service_id: row.service_id != null ? String(row.service_id) : row.service_id,
    printing_material_id:
      row.printing_material_id != null ? String(row.printing_material_id) : null,
    is_debt: isDebt,
    amount_paid: amountPaid,
    amount,
  };
}

printing.get("/jobs", async (c) => {
  const db = turso(c.env);
  const page = Math.max(1, Number(c.req.query("page") ?? 1));
  const perPage = Math.min(100, Math.max(1, Number(c.req.query("per_page") ?? 50)));
  const offset = (page - 1) * perPage;
  const search = (c.req.query("search") ?? "").trim();

  let where = "";
  const args: (string | number)[] = [];
  if (search) {
    where = `WHERE (
      LOWER(COALESCE(st.service_name, '')) LIKE ?
      OR LOWER(COALESCE(st.customer_name, '')) LIKE ?
      OR LOWER(COALESCE(st.material_type, '')) LIKE ?
    )`;
    const q = `%${search.toLowerCase()}%`;
    args.push(q, q, q);
  }

  const count = await db.execute({
    sql: `SELECT COUNT(*) AS n FROM service_transactions st ${where}`,
    args,
  });
  const total = Number(count.rows[0]?.n ?? 0);

  const res = await db.execute({
    sql: `SELECT st.*,
            CASE WHEN COALESCE(st.is_debt, 0) = 0 THEN st.amount
                 ELSE COALESCE((
                   SELECT d.paid_amount FROM debts d
                   WHERE d.service_transaction_id = st.id LIMIT 1
                 ), 0)
            END AS amount_paid,
            COALESCE((
              SELECT d.paid_amount FROM debts d
              WHERE d.service_transaction_id = st.id LIMIT 1
            ), 0) AS debt_paid
          FROM service_transactions st
          ${where}
          ORDER BY st.timestamp DESC
          LIMIT ? OFFSET ?`,
    args: [...args, perPage, offset],
  });
  const today = await db.execute(`
    SELECT
      COALESCE(SUM(CASE WHEN COALESCE(is_debt,0)=0 THEN amount ELSE 0 END), 0) AS today_earnings,
      COUNT(*) AS total_jobs_count,
      COALESCE(SUM(stock_metres_used), 0) AS material_used
    FROM service_transactions
    WHERE date(timestamp) = date('now', 'localtime')
  `);
  const rev = await db.execute(
    `SELECT COALESCE(SUM(CASE WHEN COALESCE(is_debt,0)=0 THEN amount ELSE 0 END), 0) AS total_revenue
     FROM service_transactions`,
  );
  const t = today.rows[0] ?? {};

  return c.json({
    items: res.rows.map((r) => mapJob(r as Record<string, unknown>)),
    total_count: total,
    page,
    per_page: perPage,
    today_earnings: Number(t.today_earnings ?? 0),
    total_jobs_count: Number(t.total_jobs_count ?? 0),
    material_used: Number(t.material_used ?? 0),
    total_revenue: Number(rev.rows[0]?.total_revenue ?? 0),
  });
});
printing.post("/jobs", async (c) => {
  const body = await c.req.json<{
    service_name: string;
    quantity?: number;
    price?: number;
    amount?: number;
    payment_method?: string;
    customer_name?: string;
    notes?: string | null;
    printing_material_id?: number | null;
    stock_metres_used?: number;
    material_size?: string | null;
    material_type?: string | null;
    is_debt?: number;
  }>();

  if (!body.service_name?.trim()) {
    return c.json({ error: "service_name is required" }, 400);
  }

  const metres = Number(body.stock_metres_used ?? 0);
  const amount = Number(body.amount ?? body.price ?? 0);
  if (metres <= 0 || amount <= 0) {
    return c.json({ error: "stock_metres_used and amount must be > 0" }, 400);
  }

  const db = turso(c.env);
  const mid = body.printing_material_id ?? null;

  if (mid) {
    const mat = await db.execute({
      sql: `SELECT total_metres, metres_used FROM printing_materials WHERE id = ?`,
      args: [mid],
    });
    if (!mat.rows.length) {
      return c.json({ error: "Material not found" }, 404);
    }
    const rem =
      Number(mat.rows[0].total_metres ?? 0) -
      Number(mat.rows[0].metres_used ?? 0);
    if (rem + 1e-9 < metres) {
      return c.json(
        { error: `Insufficient material: ${rem.toFixed(1)}m left` },
        400,
      );
    }
    await db.execute({
      sql: `UPDATE printing_materials
            SET metres_used = metres_used + ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?`,
      args: [metres, mid],
    });
  }

  const id = newDistributedId();
  await db.execute({
    sql: `INSERT INTO service_transactions
            (id, service_id, service_name, quantity, price, amount, payment_method,
             customer_name, notes, stock_id, stock_metres_used, material_size,
             material_type, printing_material_id, is_debt)
          VALUES (?, NULL, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?, ?, ?, ?)`,
    args: [
      id,
      body.service_name.trim(),
      body.quantity ?? 1,
      body.price ?? amount,
      amount,
      body.payment_method ?? "cash",
      body.customer_name ?? "Walk-in",
      body.notes ?? null,
      metres,
      body.material_size ?? null,
      body.material_type ?? null,
      mid,
      body.is_debt ?? 0,
    ],
  });

  const row = await db.execute({
    sql: "SELECT * FROM service_transactions WHERE id = ?",
    args: [id],
  });
  return c.json(row.rows[0], 201);
});

printing.patch("/jobs/:id", async (c) => {
  const id = asId(c.req.param("id"));
  if (id == null) return c.json({ error: "Invalid id" }, 400);
  const body = await c.req.json<Record<string, unknown>>();
  const db = turso(c.env);

  const fields: string[] = [];
  const args: (string | number | null)[] = [];
  if (body.is_debt !== undefined) {
    fields.push("is_debt = ?");
    args.push(asInt(body.is_debt, 0));
  }
  if (body.payment_method !== undefined) {
    fields.push("payment_method = ?");
    args.push(body.payment_method == null ? null : String(body.payment_method));
  }
  if (body.customer_name !== undefined) {
    fields.push("customer_name = ?");
    args.push(body.customer_name == null ? null : String(body.customer_name));
  }
  if (body.amount !== undefined) {
    fields.push("amount = ?");
    args.push(Number(body.amount));
  }
  if (!fields.length) return c.json({ error: "No fields" }, 400);
  args.push(id);
  await db.execute({
    sql: `UPDATE service_transactions SET ${fields.join(", ")} WHERE id = ?`,
    args,
  });
  return c.json({ success: true });
});

printing.delete("/jobs/:id", async (c) => {
  const id = asId(c.req.param("id"));
  if (id == null) return c.json({ error: "Invalid id" }, 400);
  const db = turso(c.env);
  await db.execute({
    sql: "DELETE FROM service_transactions WHERE id = ?",
    args: [id],
  });
  return c.json({ success: true });
});
