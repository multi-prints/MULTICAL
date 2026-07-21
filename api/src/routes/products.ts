import { Hono } from "hono";
import { newDistributedId, productNaturalKey, turso } from "../db";
import type { Env } from "../env";

type AppEnv = { Bindings: Env };

export const products = new Hono<AppEnv>();

products.get("/", async (c) => {
  const db = turso(c.env);
  const page = Math.max(1, Number(c.req.query("page") ?? 1));
  const perPage = Math.min(100, Math.max(1, Number(c.req.query("per_page") ?? 50)));
  const offset = (page - 1) * perPage;

  const count = await db.execute("SELECT COUNT(*) AS n FROM products");
  const total = Number(count.rows[0]?.n ?? 0);

  const res = await db.execute({
    sql: `SELECT id, name, product_type, color, size, selling_price, stock,
                 created_at, updated_at
          FROM products
          ORDER BY created_at DESC
          LIMIT ? OFFSET ?`,
    args: [perPage, offset],
  });

  const metrics = await db.execute(`
    SELECT
      COALESCE(SUM(stock), 0) AS total_stock_units,
      COALESCE(SUM(CASE WHEN product_type = 'life_saver' THEN stock ELSE 0 END), 0) AS life_saver_stock,
      COALESCE(SUM(CASE WHEN product_type = 'chevron' THEN stock ELSE 0 END), 0) AS chevron_stock,
      COALESCE(SUM(CASE WHEN product_type = 'stripes' THEN stock ELSE 0 END), 0) AS stripes_stock,
      COALESCE(SUM(stock * selling_price), 0) AS stock_value
    FROM products
  `);
  const m = metrics.rows[0] ?? {};

  return c.json({
    items: res.rows,
    total_count: total,
    page,
    per_page: perPage,
    total_stock_units: Number(m.total_stock_units ?? 0),
    life_saver_stock: Number(m.life_saver_stock ?? 0),
    chevron_stock: Number(m.chevron_stock ?? 0),
    stripes_stock: Number(m.stripes_stock ?? 0),
    stock_value: Number(m.stock_value ?? 0),
  });
});

products.get("/:id", async (c) => {
  const db = turso(c.env);
  const id = Number(c.req.param("id"));
  const res = await db.execute({
    sql: `SELECT id, name, product_type, color, size, selling_price, stock,
                 created_at, updated_at
          FROM products WHERE id = ?`,
    args: [id],
  });
  if (!res.rows.length) return c.json({ error: "Not found" }, 404);
  return c.json(res.rows[0]);
});

products.post("/", async (c) => {
  const body = await c.req.json<{
    name?: string;
    product_type: string;
    color?: string | null;
    size?: string | null;
    selling_price?: number;
    stock?: number;
  }>();

  if (!body.product_type?.trim()) {
    return c.json({ error: "product_type is required" }, 400);
  }

  const db = turso(c.env);
  const productType = body.product_type.trim();
  const color = body.color ?? null;
  const size = body.size ?? null;
  const key = productNaturalKey(productType, color, size);
  const stock = Math.max(0, Number(body.stock ?? 0));
  const price = Number(body.selling_price ?? 0);
  const name =
    body.name?.trim() ||
    (productType === "life_saver"
      ? "Life Saver"
      : productType === "chevron"
        ? "Chevron"
        : productType === "stripes"
          ? `${(color ?? "mixed").toString()} Stripes`
          : productType);

  // Upsert by natural key so multi-PC adds merge instead of duplicating
  const existing = await db.execute({
    sql: "SELECT id, stock FROM products WHERE natural_key = ? LIMIT 1",
    args: [key],
  });

  if (existing.rows.length) {
    const id = Number(existing.rows[0].id);
    await db.execute({
      sql: `UPDATE products
            SET stock = stock + ?, selling_price = CASE WHEN ? > 0 THEN ? ELSE selling_price END,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?`,
      args: [stock, price, price, id],
    });
    const row = await db.execute({
      sql: "SELECT * FROM products WHERE id = ?",
      args: [id],
    });
    return c.json(row.rows[0], 200);
  }

  const id = newDistributedId();
  await db.execute({
    sql: `INSERT INTO products
            (id, name, product_type, color, size, selling_price, stock, natural_key)
          VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
    args: [id, name, productType, color, size, price, stock, key],
  });
  const row = await db.execute({
    sql: "SELECT * FROM products WHERE id = ?",
    args: [id],
  });
  return c.json(row.rows[0], 201);
});

products.patch("/:id", async (c) => {
  const id = Number(c.req.param("id"));
  const body = await c.req.json<Record<string, unknown>>();
  const db = turso(c.env);

  const fields: string[] = [];
  const args: (string | number | null)[] = [];
  for (const [k, v] of Object.entries(body)) {
    if (
      ["name", "product_type", "color", "size", "selling_price", "stock"].includes(
        k,
      ) &&
      v !== undefined
    ) {
      fields.push(`${k} = ?`);
      if (v === null) args.push(null);
      else if (typeof v === "number") args.push(v);
      else args.push(String(v));
    }
  }
  if (!fields.length) return c.json({ error: "No fields to update" }, 400);

  fields.push("updated_at = CURRENT_TIMESTAMP");
  args.push(id);
  await db.execute({
    sql: `UPDATE products SET ${fields.join(", ")} WHERE id = ?`,
    args,
  });
  return c.json({ success: true });
});

/** Relative stock change — safe across concurrent tills */
products.post("/:id/adjust-stock", async (c) => {
  const id = Number(c.req.param("id"));
  const { delta } = await c.req.json<{ delta: number }>();
  if (typeof delta !== "number" || !Number.isFinite(delta)) {
    return c.json({ error: "delta must be a number" }, 400);
  }
  const db = turso(c.env);
  await db.execute({
    sql: `UPDATE products
          SET stock = MAX(0, stock + ?), updated_at = CURRENT_TIMESTAMP
          WHERE id = ?`,
    args: [delta, id],
  });
  return c.json({ success: true });
});

products.delete("/:id", async (c) => {
  const id = Number(c.req.param("id"));
  const db = turso(c.env);
  await db.execute({ sql: "DELETE FROM products WHERE id = ?", args: [id] });
  return c.json({ success: true });
});
