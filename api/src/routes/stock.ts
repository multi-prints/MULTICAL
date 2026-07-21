import { Hono } from "hono";
import { newDistributedId, stockNaturalKey, turso } from "../db";
import type { Env } from "../env";

type AppEnv = { Bindings: Env };

export const stock = new Hono<AppEnv>();

stock.get("/", async (c) => {
  const db = turso(c.env);
  const page = Math.max(1, Number(c.req.query("page") ?? 1));
  const perPage = Math.min(100, Math.max(1, Number(c.req.query("per_page") ?? 50)));
  const offset = (page - 1) * perPage;

  const count = await db.execute("SELECT COUNT(*) AS n FROM stock");
  const total = Number(count.rows[0]?.n ?? 0);

  const res = await db.execute({
    sql: `SELECT id, color, size, sticker_type, rolls, metres_per_roll,
                 total_metres, metres_used, created_at, updated_at
          FROM stock
          ORDER BY created_at DESC
          LIMIT ? OFFSET ?`,
    args: [perPage, offset],
  });

  const metrics = await db.execute(`
    SELECT
      COALESCE(SUM(rolls), 0) AS total_rolls,
      COALESCE(SUM(total_metres), 0) AS total_metres,
      COALESCE(SUM(total_metres - COALESCE(metres_used, 0)), 0) AS remaining_metres
    FROM stock
  `);
  const m = metrics.rows[0] ?? {};

  return c.json({
    items: res.rows,
    total_count: total,
    page,
    per_page: perPage,
    total_rolls: Number(m.total_rolls ?? 0),
    total_metres: Number(m.total_metres ?? 0),
    remaining_metres: Number(m.remaining_metres ?? 0),
  });
});

stock.post("/", async (c) => {
  const body = await c.req.json<{
    color: string;
    size?: string;
    sticker_type?: string;
    rolls: number;
    metres_per_roll?: number;
  }>();

  if (!body.color?.trim() || !body.rolls || body.rolls <= 0) {
    return c.json({ error: "color and positive rolls are required" }, 400);
  }

  const color = body.color.trim();
  const size = (body.size ?? "1").toString();
  const stickerType = (body.sticker_type ?? "colored").toString();
  const mpr = Number(body.metres_per_roll ?? 50);
  const rolls = Math.floor(body.rolls);
  const key = stockNaturalKey(color, size, stickerType);
  const db = turso(c.env);

  const existing = await db.execute({
    sql: "SELECT id, rolls, metres_per_roll FROM stock WHERE natural_key = ? LIMIT 1",
    args: [key],
  });

  if (existing.rows.length) {
    const id = Number(existing.rows[0].id);
    const rowMpr = Number(existing.rows[0].metres_per_roll ?? mpr);
    await db.execute({
      sql: `UPDATE stock
            SET rolls = rolls + ?,
                total_metres = total_metres + ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?`,
      args: [rolls, rolls * rowMpr, id],
    });
    const row = await db.execute({
      sql: "SELECT * FROM stock WHERE id = ?",
      args: [id],
    });
    return c.json(row.rows[0], 200);
  }

  const id = newDistributedId();
  await db.execute({
    sql: `INSERT INTO stock
            (id, color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used, natural_key)
          VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?)`,
    args: [id, color, size, stickerType, rolls, mpr, rolls * mpr, key],
  });
  const row = await db.execute({
    sql: "SELECT * FROM stock WHERE id = ?",
    args: [id],
  });
  return c.json(row.rows[0], 201);
});

stock.post("/:id/add-rolls", async (c) => {
  const id = Number(c.req.param("id"));
  const { rolls } = await c.req.json<{ rolls: number }>();
  if (!rolls || rolls <= 0) return c.json({ error: "rolls must be > 0" }, 400);

  const db = turso(c.env);
  const cur = await db.execute({
    sql: "SELECT metres_per_roll FROM stock WHERE id = ?",
    args: [id],
  });
  if (!cur.rows.length) return c.json({ error: "Not found" }, 404);
  const mpr = Number(cur.rows[0].metres_per_roll ?? 50);

  await db.execute({
    sql: `UPDATE stock
          SET rolls = rolls + ?,
              total_metres = total_metres + ?,
              updated_at = CURRENT_TIMESTAMP
          WHERE id = ?`,
    args: [rolls, rolls * mpr, id],
  });
  return c.json({ success: true });
});

stock.delete("/:id", async (c) => {
  const id = Number(c.req.param("id"));
  const db = turso(c.env);
  await db.execute({ sql: "DELETE FROM stock WHERE id = ?", args: [id] });
  return c.json({ success: true });
});
