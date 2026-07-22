import { Hono } from "hono";
import {
  asId,
  asInt,
  ensureCreatedByColumns,
  newDistributedId,
  turso,
} from "../db";
import type { Env } from "../env";
import { repairSettledSourceDebtFlags } from "./debts";

type AppEnv = { Bindings: Env };

export const sales = new Hono<AppEnv>();

function mapSale(row: Record<string, unknown>) {
  const isDebt = asInt(row.is_debt, 0);
  const amount = Number(row.amount ?? 0);
  // Match desktop: full amount when not debt; otherwise sum paid on linked debt
  const amountPaid =
    row.amount_paid != null
      ? Number(row.amount_paid)
      : isDebt === 0
        ? amount
        : Number(row.debt_paid ?? 0);
  return {
    ...row,
    id: row.id != null ? String(row.id) : row.id,
    product_id: row.product_id != null ? String(row.product_id) : null,
    stock_id: row.stock_id != null ? String(row.stock_id) : null,
    is_debt: isDebt,
    amount_paid: amountPaid,
    amount,
    created_by:
      row.created_by != null && String(row.created_by).trim()
        ? String(row.created_by)
        : null,
  };
}

sales.get("/", async (c) => {
  const db = turso(c.env);
  await ensureCreatedByColumns(db);
  await repairSettledSourceDebtFlags(db);
  const page = Math.max(1, Number(c.req.query("page") ?? 1));
  const perPage = Math.min(100, Math.max(1, Number(c.req.query("per_page") ?? 50)));
  const offset = (page - 1) * perPage;
  const search = (c.req.query("search") ?? "").trim();

  let where = "";
  const args: (string | number)[] = [];
  if (search) {
    where = `WHERE (
      LOWER(COALESCE(s.product_name, '')) LIKE ?
      OR LOWER(COALESCE(s.customer_name, '')) LIKE ?
      OR LOWER(COALESCE(s.type, '')) LIKE ?
      OR LOWER(COALESCE(s.created_by, '')) LIKE ?
    )`;
    const q = `%${search.toLowerCase()}%`;
    args.push(q, q, q, q);
  }

  const count = await db.execute({
    sql: `SELECT COUNT(*) AS n FROM sales s ${where}`,
    args,
  });
  const total = Number(count.rows[0]?.n ?? 0);

  // amount_paid mirrors local Tauri get_sales_page so Debt rows show cash so far
  const res = await db.execute({
    sql: `SELECT s.*,
            CASE WHEN COALESCE(s.is_debt, 0) = 0 THEN s.amount
                 ELSE COALESCE((SELECT d.paid_amount FROM debts d WHERE d.sale_id = s.id LIMIT 1), 0)
            END AS amount_paid,
            COALESCE((SELECT d.paid_amount FROM debts d WHERE d.sale_id = s.id LIMIT 1), 0) AS debt_paid
          FROM sales s
          ${where}
          ORDER BY s.timestamp DESC
          LIMIT ? OFFSET ?`,
    args: [...args, perPage, offset],
  });
  const metrics = await db.execute(`
    SELECT
      COALESCE(SUM(CASE WHEN date(timestamp) = date('now', 'localtime') AND COALESCE(is_debt,0)=0 THEN amount ELSE 0 END), 0) AS today_total,
      COALESCE(SUM(CASE WHEN COALESCE(is_debt,0)=0 THEN amount ELSE 0 END), 0) AS all_revenue,
      COALESCE(SUM(CASE WHEN type IN ('product','stock') OR product_id IS NOT NULL THEN 1 ELSE 0 END), 0) AS product_sales_count
    FROM sales
  `);
  const m = metrics.rows[0] ?? {};

  return c.json({
    items: res.rows.map((r) => mapSale(r as Record<string, unknown>)),
    total_count: total,
    page,
    per_page: perPage,
    today_total: Number(m.today_total ?? 0),
    all_revenue: Number(m.all_revenue ?? 0),
    product_sales_count: Number(m.product_sales_count ?? 0),
  });
});
sales.post("/", async (c) => {
  const body = await c.req.json<{
    type: string;
    product_id?: number | null;
    stock_id?: number | null;
    product_name?: string | null;
    product_type?: string | null;
    sticker_type?: string | null;
    quantity?: string | number;
    amount: number;
    payment_method?: string;
    customer_name?: string;
    is_debt?: number;
    stock_metres_used?: number;
    created_by?: string | null;
  }>();

  if (!body.type || body.amount == null) {
    return c.json({ error: "type and amount are required" }, 400);
  }

  const db = turso(c.env);
  await ensureCreatedByColumns(db);
  const id = newDistributedId();
  const qty = String(body.quantity ?? "1");
  const metres = Number(body.stock_metres_used ?? 0);

  // Deduct product stock
  if (body.product_id) {
    const q = Number(qty) || 1;
    await db.execute({
      sql: `UPDATE products
            SET stock = MAX(0, stock - ?), updated_at = CURRENT_TIMESTAMP
            WHERE id = ?`,
      args: [q, body.product_id],
    });
  }

  // Deduct sticker metres
  if (body.stock_id && metres > 0) {
    await db.execute({
      sql: `UPDATE stock
            SET metres_used = metres_used + ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?`,
      args: [metres, body.stock_id],
    });
  }

  const createdBy =
    typeof body.created_by === "string" && body.created_by.trim()
      ? body.created_by.trim()
      : null;

  await db.execute({
    sql: `INSERT INTO sales
            (id, type, product_id, stock_id, product_name, product_type, sticker_type,
             quantity, amount, payment_method, customer_name, is_debt, created_by)
          VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
    args: [
      id,
      body.type,
      body.product_id ?? null,
      body.stock_id ?? null,
      body.product_name ?? null,
      body.product_type ?? null,
      body.sticker_type ?? null,
      qty,
      body.amount,
      body.payment_method ?? "cash",
      body.customer_name ?? "Walk-in",
      body.is_debt ?? 0,
      createdBy,
    ],
  });

  const row = await db.execute({
    sql: "SELECT * FROM sales WHERE id = ?",
    args: [id],
  });
  return c.json(row.rows[0], 201);
});

sales.patch("/:id", async (c) => {
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
    sql: `UPDATE sales SET ${fields.join(", ")} WHERE id = ?`,
    args,
  });
  return c.json({ success: true });
});

sales.delete("/:id", async (c) => {
  const id = asId(c.req.param("id"));
  if (id == null) return c.json({ error: "Invalid id" }, 400);
  const db = turso(c.env);
  await db.execute({ sql: "DELETE FROM sales WHERE id = ?", args: [id] });
  return c.json({ success: true });
});
