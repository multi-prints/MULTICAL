import type { Context, Next } from "hono";
import type { Env } from "./env";

type AppEnv = { Bindings: Env };

/**
 * Desktop clients send: Authorization: Bearer <API_SECRET>
 * Set API_SECRET with: wrangler secret put API_SECRET
 */
export async function requireApiSecret(
  c: Context<AppEnv>,
  next: Next,
): Promise<Response | void> {
  const secret = c.env.API_SECRET;
  if (!secret) {
    return c.json(
      { error: "API_SECRET is not configured on the Worker" },
      500,
    );
  }

  const header = c.req.header("Authorization") ?? "";
  const token = header.startsWith("Bearer ")
    ? header.slice("Bearer ".length).trim()
    : header.trim();

  if (!token || token !== secret) {
    return c.json({ error: "Unauthorized" }, 401);
  }

  await next();
}
