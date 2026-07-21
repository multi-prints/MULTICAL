import { Hono } from "hono";
import { newDistributedId, turso } from "../db";
import type { Env } from "../env";
import { hashPassword, randomToken, verifyPassword } from "../password";

type AppEnv = { Bindings: Env };

export const users = new Hono<AppEnv>();
export const auth = new Hono<AppEnv>();

async function ensureSessionsTable(
  db: ReturnType<typeof turso>,
): Promise<void> {
  await db.execute(`
    CREATE TABLE IF NOT EXISTS api_sessions (
      token TEXT PRIMARY KEY,
      username TEXT NOT NULL,
      role TEXT NOT NULL,
      permissions TEXT,
      created_at TEXT DEFAULT CURRENT_TIMESTAMP
    )
  `);
}

function parsePermissions(
  raw: unknown,
  role: string,
): string[] {
  if (typeof raw === "string" && raw.trim()) {
    try {
      const p = JSON.parse(raw);
      if (Array.isArray(p)) return p.map(String);
    } catch {
      /* ignore */
    }
  }
  if (role === "admin") return ["all"];
  return [
    "view_printing",
    "edit_printing",
    "convert_to_debt",
    "view_sales",
    "edit_sales",
  ];
}

auth.post("/login", async (c) => {
  const body = await c.req.json<{ username?: string; password?: string }>();
  const username = (body.username ?? "").trim();
  const password = body.password ?? "";
  if (!username || !password) {
    return c.json({
      success: false,
      token: null,
      user: null,
      error: "Username and password required",
    });
  }

  const db = turso(c.env);
  await ensureSessionsTable(db);

  const res = await db.execute({
    sql: `SELECT username, password_hash, role, permissions FROM users WHERE username = ?`,
    args: [username],
  });
  if (!res.rows.length) {
    return c.json({
      success: false,
      token: null,
      user: null,
      error: "Invalid username or password",
    });
  }

  const row = res.rows[0];
  const ok = await verifyPassword(
    password,
    String(row.password_hash ?? ""),
  );
  if (!ok) {
    return c.json({
      success: false,
      token: null,
      user: null,
      error: "Invalid username or password",
    });
  }

  const role = String(row.role ?? "employee");
  const permissions = parsePermissions(row.permissions, role);
  const token = randomToken();

  await db.execute({
    sql: `INSERT INTO api_sessions (token, username, role, permissions, created_at)
          VALUES (?, ?, ?, ?, ?)`,
    args: [
      token,
      username,
      role,
      JSON.stringify(permissions),
      new Date().toISOString(),
    ],
  });

  return c.json({
    success: true,
    token,
    user: { username, role, permissions },
    error: null,
  });
});

auth.post("/logout", async (c) => {
  const body = await c
    .req.json<{ token?: string }>()
    .catch(() => ({}) as { token?: string });
  const token = (body.token ?? c.req.header("X-Session-Token") ?? "").trim();
  if (token) {
    const db = turso(c.env);
    await ensureSessionsTable(db);
    await db.execute({
      sql: "DELETE FROM api_sessions WHERE token = ?",
      args: [token],
    });
  }
  return c.json({ success: true });
});

auth.post("/validate", async (c) => {
  const body = await c.req.json<{ token?: string }>();
  const token = (body.token ?? "").trim();
  if (!token) return c.json(false);
  const db = turso(c.env);
  await ensureSessionsTable(db);
  const res = await db.execute({
    sql: "SELECT token FROM api_sessions WHERE token = ?",
    args: [token],
  });
  return c.json(res.rows.length > 0);
});

auth.post("/session", async (c) => {
  const body = await c.req.json<{ token?: string }>();
  const token = (body.token ?? "").trim();
  if (!token) return c.json({ success: false, session: null });
  const db = turso(c.env);
  await ensureSessionsTable(db);
  const res = await db.execute({
    sql: "SELECT username, role, permissions FROM api_sessions WHERE token = ?",
    args: [token],
  });
  if (!res.rows.length) return c.json({ success: false, session: null });
  const row = res.rows[0];
  const role = String(row.role ?? "employee");
  return c.json({
    success: true,
    session: {
      username: String(row.username),
      role,
      permissions: parsePermissions(row.permissions, role),
    },
  });
});

users.get("/", async (c) => {
  const db = turso(c.env);
  const res = await db.execute(
    `SELECT id, username, role, created_at FROM users ORDER BY username ASC`,
  );
  return c.json({ items: res.rows });
});

users.post("/", async (c) => {
  const body = await c.req.json<{
    username: string;
    password: string;
    role?: string;
  }>();
  const username = (body.username ?? "").trim();
  const password = body.password ?? "";
  const role = (body.role ?? "employee").trim() || "employee";
  if (!username || !password) {
    return c.json({ error: "username and password required" }, 400);
  }

  const db = turso(c.env);
  const exists = await db.execute({
    sql: "SELECT id FROM users WHERE username = ?",
    args: [username],
  });
  if (exists.rows.length) {
    return c.json({ error: "Username already exists" }, 400);
  }

  const hash = await hashPassword(password);
  const permissions =
    role === "admin"
      ? '["all"]'
      : '["view_printing","edit_printing","convert_to_debt","view_sales","edit_sales"]';
  const id = newDistributedId();
  await db.execute({
    sql: `INSERT INTO users (id, username, password_hash, role, permissions)
          VALUES (?, ?, ?, ?, ?)`,
    args: [id, username, hash, role, permissions],
  });
  return c.json({ success: true });
});

users.post("/update-password", async (c) => {
  const body = await c.req.json<{
    username: string;
    old_password: string;
    new_password: string;
  }>();
  const db = turso(c.env);
  const res = await db.execute({
    sql: "SELECT password_hash FROM users WHERE username = ?",
    args: [body.username],
  });
  if (!res.rows.length) return c.json({ error: "User not found" }, 404);
  const ok = await verifyPassword(
    body.old_password,
    String(res.rows[0].password_hash),
  );
  if (!ok) return c.json({ error: "Current password is wrong" }, 400);
  const hash = await hashPassword(body.new_password);
  await db.execute({
    sql: `UPDATE users SET password_hash = ?, updated_at = CURRENT_TIMESTAMP WHERE username = ?`,
    args: [hash, body.username],
  });
  return c.json({ success: true });
});

users.post("/update-username", async (c) => {
  const body = await c.req.json<{
    old_username: string;
    new_username: string;
  }>();
  const neu = (body.new_username ?? "").trim();
  if (!neu) return c.json({ error: "new_username required" }, 400);
  const db = turso(c.env);
  await db.execute({
    sql: `UPDATE users SET username = ?, updated_at = CURRENT_TIMESTAMP WHERE username = ?`,
    args: [neu, body.old_username],
  });
  await db.execute({
    sql: `UPDATE api_sessions SET username = ? WHERE username = ?`,
    args: [neu, body.old_username],
  }).catch(() => undefined);
  return c.json({ success: true });
});

users.delete("/:username", async (c) => {
  const username = decodeURIComponent(c.req.param("username"));
  if (username === "admin") {
    return c.json({ error: "Cannot delete admin" }, 400);
  }
  const db = turso(c.env);
  await db.execute({
    sql: "DELETE FROM api_sessions WHERE username = ?",
    args: [username],
  }).catch(() => undefined);
  await db.execute({
    sql: "DELETE FROM users WHERE username = ?",
    args: [username],
  });
  return c.json({ success: true });
});
