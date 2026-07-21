import { Hono } from "hono";
import { turso } from "../db";
import type { Env } from "../env";

type AppEnv = { Bindings: Env };

export const dashboard = new Hono<AppEnv>();

dashboard.get("/summary", async (c) => {
  const db = turso(c.env);

  const revenue = await db.execute(`
    SELECT
      COALESCE((SELECT SUM(amount) FROM sales WHERE COALESCE(is_debt, 0) = 0), 0) +
      COALESCE((SELECT SUM(amount) FROM service_transactions WHERE COALESCE(is_debt, 0) = 0), 0) +
      COALESCE((SELECT SUM(amount) FROM debt_payments), 0) AS total_revenue
  `);

  const todaySales = await db.execute(`
    SELECT COUNT(*) AS cnt,
      COALESCE(SUM(CASE WHEN COALESCE(is_debt, 0) = 0 THEN amount ELSE 0 END), 0) AS rev
    FROM sales
    WHERE date(timestamp) = date('now', 'localtime')
  `);

  const todayJobs = await db.execute(`
    SELECT COUNT(*) AS cnt,
      COALESCE(SUM(CASE WHEN COALESCE(is_debt, 0) = 0 THEN amount ELSE 0 END), 0) AS rev
    FROM service_transactions
    WHERE date(timestamp) = date('now', 'localtime')
  `);

  const debts = await db.execute(`
    SELECT COALESCE(SUM(remaining_amount), 0) AS outstanding,
           COUNT(*) AS pending_count
    FROM debts WHERE status = 'pending'
  `);

  const recent = await db.execute(`
    SELECT name, date, amount, is_debt, type_label FROM (
      SELECT COALESCE(product_name, type) AS name,
             COALESCE(substr(timestamp, 1, 10), '') AS date,
             amount,
             CASE WHEN COALESCE(is_debt, 0) > 0 THEN 1 ELSE 0 END AS is_debt,
             'Sale' AS type_label,
             COALESCE(timestamp, '') AS sort_ts
      FROM sales
      UNION ALL
      SELECT service_name AS name,
             COALESCE(substr(timestamp, 1, 10), '') AS date,
             amount,
             CASE WHEN COALESCE(is_debt, 0) > 0 THEN 1 ELSE 0 END AS is_debt,
             'Printing' AS type_label,
             COALESCE(timestamp, '') AS sort_ts
      FROM service_transactions
    )
    ORDER BY sort_ts DESC
    LIMIT 8
  `);

  const activity = await db.execute(`
    SELECT item_type, text, time FROM (
      SELECT 'sale' AS item_type,
             COALESCE(product_name, type) || ' — KSh ' || CAST(CAST(amount AS INTEGER) AS TEXT) AS text,
             COALESCE(substr(timestamp, 12, 5), '') AS time,
             COALESCE(timestamp, '') AS sort_ts
      FROM sales
      UNION ALL
      SELECT 'debt' AS item_type,
             'Debt: ' || customer_name || ' — KSh ' || CAST(CAST(amount AS INTEGER) AS TEXT) AS text,
             COALESCE(substr(created_at, 12, 5), '') AS time,
             COALESCE(created_at, '') AS sort_ts
      FROM debts
      UNION ALL
      SELECT 'printing' AS item_type,
             service_name || ' — KSh ' || CAST(CAST(amount AS INTEGER) AS TEXT) AS text,
             COALESCE(substr(timestamp, 12, 5), '') AS time,
             COALESCE(timestamp, '') AS sort_ts
      FROM service_transactions
    )
    ORDER BY sort_ts DESC
    LIMIT 12
  `);

  const top = await db.execute(`
    SELECT product_id, COALESCE(product_name, type) AS name,
           COUNT(*) AS quantity
    FROM sales
    WHERE product_id IS NOT NULL OR product_name IS NOT NULL
    GROUP BY COALESCE(product_id, -1), COALESCE(product_name, type)
    ORDER BY quantity DESC
    LIMIT 5
  `);

  const ts = Number(todaySales.rows[0]?.cnt ?? 0);
  const tj = Number(todayJobs.rows[0]?.cnt ?? 0);
  const today_revenue =
    Number(todaySales.rows[0]?.rev ?? 0) + Number(todayJobs.rows[0]?.rev ?? 0);

  return c.json({
    total_revenue: Number(revenue.rows[0]?.total_revenue ?? 0),
    today_sales_count: ts + tj,
    today_revenue,
    outstanding_debts: Number(debts.rows[0]?.outstanding ?? 0),
    pending_debts_count: Number(debts.rows[0]?.pending_count ?? 0),
    recent_transactions: recent.rows.map((r) => ({
      name: String(r.name ?? ""),
      date: String(r.date ?? ""),
      amount: Number(r.amount ?? 0),
      is_debt: Number(r.is_debt ?? 0) > 0,
      type_label: String(r.type_label ?? ""),
    })),
    activity_items: activity.rows.map((r) => ({
      item_type: String(r.item_type ?? ""),
      text: String(r.text ?? ""),
      time: String(r.time ?? ""),
    })),
    top_products: top.rows.map((r) => ({
      product_id: r.product_id != null ? Number(r.product_id) : null,
      name: String(r.name ?? ""),
      quantity: Number(r.quantity ?? 0),
    })),
  });
});

dashboard.get("/chart", async (c) => {
  const period = c.req.query("period") ?? "week";
  const db = turso(c.env);

  // Simple last-7-days revenue by day (week default); month uses day-of-month
  const days = period === "year" ? 12 : period === "month" ? 30 : 7;

  if (period === "year") {
    const res = await db.execute(`
      SELECT strftime('%Y-%m', ts) AS label, SUM(amount) AS amount, COUNT(*) AS sales_count
      FROM (
        SELECT timestamp AS ts, amount FROM sales WHERE COALESCE(is_debt,0)=0
        UNION ALL
        SELECT timestamp AS ts, amount FROM service_transactions WHERE COALESCE(is_debt,0)=0
      )
      WHERE ts IS NOT NULL AND date(ts) >= date('now', 'localtime', '-365 days')
      GROUP BY strftime('%Y-%m', ts)
      ORDER BY label ASC
    `);
    const debtRes = await db.execute(`
      SELECT strftime('%Y-%m', created_at) AS label,
             COALESCE(SUM(remaining_amount), 0) AS debt_amount
      FROM debts
      WHERE status = 'pending' AND created_at IS NOT NULL
        AND date(created_at) >= date('now', 'localtime', '-365 days')
      GROUP BY strftime('%Y-%m', created_at)
    `);
    const debtMap = new Map(
      debtRes.rows.map((r) => [String(r.label), Number(r.debt_amount ?? 0)]),
    );
    return c.json(
      res.rows.map((r) => ({
        label: String(r.label ?? ""),
        amount: Number(r.amount ?? 0),
        sales_count: Number(r.sales_count ?? 0),
        debt_amount: debtMap.get(String(r.label)) ?? 0,
      })),
    );
  }

  const res = await db.execute({
    sql: `
      SELECT date(ts) AS label, SUM(amount) AS amount, COUNT(*) AS sales_count
      FROM (
        SELECT timestamp AS ts, amount FROM sales WHERE COALESCE(is_debt,0)=0
        UNION ALL
        SELECT timestamp AS ts, amount FROM service_transactions WHERE COALESCE(is_debt,0)=0
      )
      WHERE ts IS NOT NULL AND date(ts) >= date('now', 'localtime', ?)
      GROUP BY date(ts)
      ORDER BY label ASC
    `,
    args: [`-${days} days`],
  });

  return c.json(
    res.rows.map((r) => ({
      label: String(r.label ?? ""),
      amount: Number(r.amount ?? 0),
      sales_count: Number(r.sales_count ?? 0),
      debt_amount: 0,
    })),
  );
});
