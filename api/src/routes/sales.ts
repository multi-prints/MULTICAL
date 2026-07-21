import { Hono } from "hono";
import { newDistributedId, turso } from "../db";
import type { Env } from "../env";

type AppEnv = { Bindings: Env };

export const sales = new Hono<AppEnv>();

sales.get("/", async (c) => {
  const db = turso(c.env);
  const page = Math.max(1, Number(c.req.query("page") ?? 1));
  const perPage = Math.min(100, Math.max(1, Number(c.req.query("per_page") ?? 50)));
  const offset = (page - 1) * perPage;
  const search = (c.req.query("search") ?? "").trim();

  let where = "";
  const args: (string | number)[] = [];
  if (search) {
    where = `WHERE (
      LOWER(COALESCE(product_name, '')) LIKE ?
      OR LOWER(COALESCE(customer_name, '')) LIKE ?
      OR LOWER(COALESCE(type, '')) LIKE ?
    )`;
    const q = `%${search.toLowerCase()}%`;
    args.push(q, q, q);
  }

  const count = await db.execute({
    sql: `SELECT COUNT(*) AS n FROM sales ${where}`,
    args,
  });
  const total = Number(count.rows[0]?.n ?? 0);

  const res = await db.execute({
    sql: `SELECT * FROM sales ${where}
          ORDER BY timestamp DESC
          LIMIT ? OFFSET ?`,
    args: [...args, perPage, offset],
  });

  const metrics = await db.execute(`
    SELECT
      COALESCE(SUM(CASE WHEN date(timestamp) = date('now', 'localtime') THEN amount ELSE 0 END), 0) AS today_total,
      COALESCE(SUM(amount), 0) AS all_revenue,
      COALESCE(SUM(CASE WHEN type IN ('product','stock') OR product_id IS NOT NULL THEN 1 ELSE 0 END), 0) AS product_sales_count
    FROM sales
  `);
  const m = metrics.rows[0] ?? {};

  return c.json({
    items: res.rows,
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
  }>();

  if (!body.type || body.amount == null) {
    return c.json({ error: "type and amount are required" }, 400);
  }

  const db = turso(c.env);
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

  await db.execute({
    sql: `INSERT INTO sales
            (id, type, product_id, stock_id, product_name, product_type, sticker_type,
             quantity, amount, payment_method, customer_name, is_debt)
          VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
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
    ],
  });

  const row = await db.execute({
    sql: "SELECT * FROM sales WHERE id = ?",
    args: [id],
  });
  return c.json(row.rows[0], 201);
});

sales.delete("/:id", async (c) => {
  const id = Number(c.req.param("id"));
  const db = turso(c.env);
  await db.execute({ sql: "DELETE FROM sales WHERE id = ?", args: [id] });
  return c.json({ success: true });
});
