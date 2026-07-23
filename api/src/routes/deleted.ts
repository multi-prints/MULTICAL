import { Hono } from "hono";
import { ensureDeletedRecordsTable, turso } from "../db";
import type { Env } from "../env";

type AppEnv = { Bindings: Env };

/**
 * Admin audit log of sales / printing jobs removed by staff.
 * GET /v1/deleted?source_kind=sale|printing&search=&page=&per_page=
 */
export const deleted = new Hono<AppEnv>();

deleted.get("/", async (c) => {
  const db = turso(c.env);
  await ensureDeletedRecordsTable(db);

  const page = Math.max(1, Number(c.req.query("page") ?? 1));
  const perPage = Math.min(
    200,
    Math.max(1, Number(c.req.query("per_page") ?? 50)),
  );
  const offset = (page - 1) * perPage;
  const kind = (c.req.query("source_kind") ?? "").trim().toLowerCase();
  const search = (c.req.query("search") ?? "").trim().toLowerCase();

  const where: string[] = [];
  const args: (string | number)[] = [];

  if (kind === "sale" || kind === "printing") {
    where.push("source_kind = ?");
    args.push(kind);
  }
  if (search) {
    const q = `%${search}%`;
    where.push(
      `(LOWER(COALESCE(summary, '')) LIKE ?
        OR LOWER(COALESCE(customer_name, '')) LIKE ?
        OR LOWER(COALESCE(created_by, '')) LIKE ?
        OR LOWER(COALESCE(deleted_by, '')) LIKE ?)`,
    );
    args.push(q, q, q, q);
  }

  const whereSql = where.length ? `WHERE ${where.join(" AND ")}` : "";

  const count = await db.execute({
    sql: `SELECT COUNT(*) AS cnt FROM deleted_records ${whereSql}`,
    args,
  });
  const total_count = Number(count.rows[0]?.cnt ?? 0);

  const list = await db.execute({
    sql: `SELECT id, source_kind, original_id, summary, amount, customer_name,
                 created_by, deleted_by, deleted_at, original_timestamp, payload
          FROM deleted_records
          ${whereSql}
          ORDER BY deleted_at DESC
          LIMIT ? OFFSET ?`,
    args: [...args, perPage, offset],
  });

  return c.json({
    items: list.rows.map((r) => ({
      id: r.id != null ? String(r.id) : r.id,
      source_kind: String(r.source_kind ?? ""),
      original_id: r.original_id != null ? String(r.original_id) : r.original_id,
      summary: String(r.summary ?? ""),
      amount: Number(r.amount ?? 0),
      customer_name: r.customer_name != null ? String(r.customer_name) : null,
      created_by: r.created_by != null ? String(r.created_by) : null,
      deleted_by: String(r.deleted_by ?? ""),
      deleted_at: r.deleted_at != null ? String(r.deleted_at) : null,
      original_timestamp:
        r.original_timestamp != null ? String(r.original_timestamp) : null,
      payload: String(r.payload ?? "{}"),
    })),
    total_count,
  });
});
