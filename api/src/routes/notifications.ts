import { Hono } from "hono";
import { turso } from "../db";
import type { Env } from "../env";
import {
  debtSelect,
  mapDebt,
  repairSettledSourceDebtFlags,
} from "./debts";

type AppEnv = { Bindings: Env };

/**
 * Titlebar / admin notification feed for multi-PC clients.
 *
 * Desktop shows overdue debts in the bell dropdown and OS toast.
 * This endpoint is the Workers equivalent of local `get_overdue_debts`.
 */
export const notifications = new Hono<AppEnv>();

notifications.get("/", async (c) => {
  const db = turso(c.env);
  // Self-heal badges for debts paid before Worker synced is_debt=2.
  await repairSettledSourceDebtFlags(db);
  const res = await db.execute(
    `${debtSelect}
     WHERE d.status = 'pending' AND d.due_date IS NOT NULL
       AND date(d.due_date) < date('now', 'localtime')
     ORDER BY d.due_date ASC`,
  );
  const items = res.rows.map((r) => mapDebt(r as Record<string, unknown>));
  const overdue_total = items.reduce(
    (sum, d) =>
      sum + Number((d as { remaining_amount?: number }).remaining_amount ?? 0),
    0,
  );

  return c.json({
    items,
    overdue_count: items.length,
    overdue_total,
    // Reserved for future kinds (low stock, due-soon, etc.)
    kinds: ["overdue_debt"] as const,
  });
});
