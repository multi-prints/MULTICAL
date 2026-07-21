import { Hono } from "hono";
import { newDistributedId, turso } from "../db";
import type { Env } from "../env";

type AppEnv = { Bindings: Env };

export const services = new Hono<AppEnv>();

services.get("/", async (c) => {
  const db = turso(c.env);
  const activeOnly = c.req.query("active") === "1";
  const res = await db.execute(
    activeOnly
      ? `SELECT * FROM services WHERE is_active = 1 ORDER BY name ASC`
      : `SELECT * FROM services ORDER BY name ASC`,
  );
  return c.json({ items: res.rows });
});

services.get("/:id", async (c) => {
  const id = Number(c.req.param("id"));
  const db = turso(c.env);
  const res = await db.execute({
    sql: "SELECT * FROM services WHERE id = ?",
    args: [id],
  });
  if (!res.rows.length) return c.json(null);
  return c.json(res.rows[0]);
});

services.post("/", async (c) => {
  const body = await c.req.json<{
    name: string;
    description?: string | null;
    price?: number | null;
    unit?: string | null;
    is_active?: number;
  }>();
  if (!body.name?.trim()) return c.json({ error: "name required" }, 400);
  const id = newDistributedId();
  const db = turso(c.env);
  await db.execute({
    sql: `INSERT INTO services (id, name, description, price, unit, uses_stock, is_active)
          VALUES (?, ?, ?, ?, ?, 0, ?)`,
    args: [
      id,
      body.name.trim(),
      body.description ?? null,
      body.price ?? 0,
      body.unit ?? null,
      body.is_active ?? 1,
    ],
  });
  const row = await db.execute({
    sql: "SELECT * FROM services WHERE id = ?",
    args: [id],
  });
  return c.json(row.rows[0], 201);
});

services.patch("/:id", async (c) => {
  const id = Number(c.req.param("id"));
  const body = await c.req.json<Record<string, unknown>>();
  const fields: string[] = [];
  const args: (string | number | null)[] = [];
  for (const k of ["name", "description", "price", "unit", "is_active", "uses_stock"]) {
    if (body[k] !== undefined) {
      fields.push(`${k} = ?`);
      const v = body[k];
      if (v === null) args.push(null);
      else if (typeof v === "number") args.push(v);
      else args.push(String(v));
    }
  }
  if (!fields.length) return c.json({ error: "No fields" }, 400);
  fields.push("updated_at = CURRENT_TIMESTAMP");
  args.push(id);
  const db = turso(c.env);
  await db.execute({
    sql: `UPDATE services SET ${fields.join(", ")} WHERE id = ?`,
    args,
  });
  return c.json({ success: true });
});

services.delete("/:id", async (c) => {
  const id = Number(c.req.param("id"));
  const db = turso(c.env);
  await db.execute({ sql: "DELETE FROM services WHERE id = ?", args: [id] });
  return c.json({ success: true });
});
