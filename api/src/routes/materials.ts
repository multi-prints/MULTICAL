import { Hono } from "hono";
import { newDistributedId, turso } from "../db";
import type { Env } from "../env";

type AppEnv = { Bindings: Env };

export const materials = new Hono<AppEnv>();

materials.get("/", async (c) => {
  const db = turso(c.env);
  const res = await db.execute(`
    SELECT id, name, material_type, width, rolls, metres_per_roll,
           total_metres, metres_used, color, created_at, updated_at
    FROM printing_materials
    ORDER BY created_at DESC
  `);
  return c.json({ items: res.rows });
});

materials.post("/", async (c) => {
  const body = await c.req.json<{
    name: string;
    material_type?: string;
    width: number;
    rolls: number;
    metres_per_roll: number;
    color?: string | null;
  }>();

  const name = body.name?.trim();
  const width = Number(body.width);
  const rolls = Math.floor(Number(body.rolls));
  const mpr = Number(body.metres_per_roll);

  if (!name || width <= 0 || rolls <= 0 || mpr <= 0) {
    return c.json(
      { error: "name, width, rolls, and metres_per_roll are required" },
      400,
    );
  }

  // material_type stores name for display (desktop no longer uses "Custom")
  const materialType = (body.material_type ?? name).trim() || name;
  const id = newDistributedId();
  const db = turso(c.env);

  await db.execute({
    sql: `INSERT INTO printing_materials
            (id, name, material_type, width, rolls, metres_per_roll,
             total_metres, metres_used, color, natural_key)
          VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?, ?)`,
    args: [
      id,
      name,
      materialType,
      width,
      rolls,
      mpr,
      rolls * mpr,
      body.color ?? null,
      name.toLowerCase(),
    ],
  });

  const row = await db.execute({
    sql: "SELECT * FROM printing_materials WHERE id = ?",
    args: [id],
  });
  return c.json(row.rows[0], 201);
});

materials.post("/:id/add-rolls", async (c) => {
  const id = Number(c.req.param("id"));
  const { rolls } = await c.req.json<{ rolls: number }>();
  if (!rolls || rolls <= 0) return c.json({ error: "rolls must be > 0" }, 400);

  const db = turso(c.env);
  const cur = await db.execute({
    sql: "SELECT metres_per_roll FROM printing_materials WHERE id = ?",
    args: [id],
  });
  if (!cur.rows.length) return c.json({ error: "Not found" }, 404);
  const mpr = Number(cur.rows[0].metres_per_roll ?? 50);

  await db.execute({
    sql: `UPDATE printing_materials
          SET rolls = rolls + ?,
              total_metres = total_metres + ?,
              updated_at = CURRENT_TIMESTAMP
          WHERE id = ?`,
    args: [rolls, rolls * mpr, id],
  });
  return c.json({ success: true });
});

materials.delete("/:id", async (c) => {
  const id = Number(c.req.param("id"));
  const db = turso(c.env);
  await db.execute({
    sql: "DELETE FROM printing_materials WHERE id = ?",
    args: [id],
  });
  return c.json({ success: true });
});
