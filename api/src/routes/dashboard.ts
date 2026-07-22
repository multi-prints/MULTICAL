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

/**
 * Chart buckets aligned with the desktop Tauri queries:
 * - week: last 7 calendar days, labels Mon/Tue/…
 * - month: last 4 weeks, labels "22–28 Jun"
 * - year: last 12 months, labels Jan/Feb/…
 * Always returns a full series (zeros included) so the UI is not sparse.
 */
dashboard.get("/chart", async (c) => {
  const period = c.req.query("period") ?? "week";
  const db = turso(c.env);

  if (period === "year") {
    const res = await db.execute(`
      WITH RECURSIVE months(idx, month_start) AS (
        SELECT 0, DATE('now', 'localtime', 'start of month')
        UNION ALL
        SELECT idx + 1, DATE(month_start, '-1 month')
        FROM months
        WHERE idx < 11
      ), revenues AS (
        SELECT strftime('%Y-%m', timestamp) AS ym, amount FROM sales WHERE COALESCE(is_debt, 0) = 0
        UNION ALL
        SELECT strftime('%Y-%m', timestamp) AS ym, amount FROM service_transactions WHERE COALESCE(is_debt, 0) = 0
        UNION ALL
        SELECT strftime('%Y-%m', payment_date) AS ym, amount FROM debt_payments
      ), tx_counts AS (
        SELECT strftime('%Y-%m', timestamp) AS ym FROM sales
        UNION ALL
        SELECT strftime('%Y-%m', timestamp) AS ym FROM service_transactions
      )
      SELECT CASE strftime('%m', month_start)
               WHEN '01' THEN 'Jan' WHEN '02' THEN 'Feb' WHEN '03' THEN 'Mar'
               WHEN '04' THEN 'Apr' WHEN '05' THEN 'May' WHEN '06' THEN 'Jun'
               WHEN '07' THEN 'Jul' WHEN '08' THEN 'Aug' WHEN '09' THEN 'Sep'
               WHEN '10' THEN 'Oct' WHEN '11' THEN 'Nov' WHEN '12' THEN 'Dec'
             END AS label,
             COALESCE((SELECT SUM(amount) FROM revenues r WHERE r.ym = strftime('%Y-%m', m.month_start)), 0) AS amount,
             COALESCE((SELECT COUNT(*) FROM tx_counts t WHERE t.ym = strftime('%Y-%m', m.month_start)), 0) AS sales_count,
             COALESCE((
               SELECT SUM(remaining_amount) FROM debts d
               WHERE d.status = 'pending'
                 AND strftime('%Y-%m', d.created_at) = strftime('%Y-%m', m.month_start)
             ), 0) AS debt_amount,
             idx
      FROM months m
      ORDER BY idx DESC
    `);
    return c.json(
      res.rows.map((r) => ({
        label: String(r.label ?? ""),
        amount: Number(r.amount ?? 0),
        sales_count: Number(r.sales_count ?? 0),
        debt_amount: Number(r.debt_amount ?? 0),
      })),
    );
  }

  if (period === "month") {
    const res = await db.execute(`
      WITH RECURSIVE weeks(idx, start_date, end_date) AS (
        SELECT 0, DATE('now', 'localtime', '-6 days'), DATE('now', 'localtime')
        UNION ALL
        SELECT idx + 1,
               DATE(start_date, '-7 days'),
               DATE(end_date, '-7 days')
        FROM weeks
        WHERE idx < 3
      ), revenues AS (
        SELECT DATE(timestamp) AS tx_date, amount FROM sales WHERE COALESCE(is_debt, 0) = 0
        UNION ALL
        SELECT DATE(timestamp) AS tx_date, amount FROM service_transactions WHERE COALESCE(is_debt, 0) = 0
        UNION ALL
        SELECT DATE(payment_date) AS tx_date, amount FROM debt_payments
      ), tx_counts AS (
        SELECT DATE(timestamp) AS tx_date FROM sales
        UNION ALL
        SELECT DATE(timestamp) AS tx_date FROM service_transactions
      )
      SELECT strftime('%d', start_date) || '–' || strftime('%d', end_date) || ' ' ||
             CASE strftime('%m', end_date)
               WHEN '01' THEN 'Jan' WHEN '02' THEN 'Feb' WHEN '03' THEN 'Mar'
               WHEN '04' THEN 'Apr' WHEN '05' THEN 'May' WHEN '06' THEN 'Jun'
               WHEN '07' THEN 'Jul' WHEN '08' THEN 'Aug' WHEN '09' THEN 'Sep'
               WHEN '10' THEN 'Oct' WHEN '11' THEN 'Nov' WHEN '12' THEN 'Dec'
             END AS label,
             COALESCE((SELECT SUM(amount) FROM revenues r WHERE r.tx_date BETWEEN w.start_date AND w.end_date), 0) AS amount,
             COALESCE((SELECT COUNT(*) FROM tx_counts t WHERE t.tx_date BETWEEN w.start_date AND w.end_date), 0) AS sales_count,
             COALESCE((
               SELECT SUM(remaining_amount) FROM debts d
               WHERE d.status = 'pending'
                 AND DATE(d.created_at) BETWEEN w.start_date AND w.end_date
             ), 0) AS debt_amount,
             idx
      FROM weeks w
      ORDER BY idx DESC
    `);
    return c.json(
      res.rows.map((r) => ({
        label: String(r.label ?? ""),
        amount: Number(r.amount ?? 0),
        sales_count: Number(r.sales_count ?? 0),
        debt_amount: Number(r.debt_amount ?? 0),
      })),
    );
  }

  // week (default): last 7 days — full series with weekday labels
  const res = await db.execute(`
    WITH RECURSIVE days(idx, day_date) AS (
      SELECT 0, DATE('now', 'localtime')
      UNION ALL
      SELECT idx + 1, DATE(day_date, '-1 day')
      FROM days
      WHERE idx < 6
    ), revenues AS (
      SELECT DATE(timestamp) AS tx_date, amount FROM sales WHERE COALESCE(is_debt, 0) = 0
      UNION ALL
      SELECT DATE(timestamp) AS tx_date, amount FROM service_transactions WHERE COALESCE(is_debt, 0) = 0
      UNION ALL
      SELECT DATE(payment_date) AS tx_date, amount FROM debt_payments
    ), tx_counts AS (
      SELECT DATE(timestamp) AS tx_date FROM sales
      UNION ALL
      SELECT DATE(timestamp) AS tx_date FROM service_transactions
    )
    SELECT CASE strftime('%w', day_date)
             WHEN '0' THEN 'Sun' WHEN '1' THEN 'Mon' WHEN '2' THEN 'Tue'
             WHEN '3' THEN 'Wed' WHEN '4' THEN 'Thu' WHEN '5' THEN 'Fri'
             WHEN '6' THEN 'Sat'
           END AS label,
           COALESCE((SELECT SUM(amount) FROM revenues r WHERE r.tx_date = d.day_date), 0) AS amount,
           COALESCE((SELECT COUNT(*) FROM tx_counts t WHERE t.tx_date = d.day_date), 0) AS sales_count,
           COALESCE((
             SELECT SUM(remaining_amount) FROM debts dbt
             WHERE dbt.status = 'pending'
               AND DATE(dbt.created_at) = d.day_date
           ), 0) AS debt_amount,
           idx
    FROM days d
    ORDER BY idx DESC
  `);

  return c.json(
    res.rows.map((r) => ({
      label: String(r.label ?? ""),
      amount: Number(r.amount ?? 0),
      sales_count: Number(r.sales_count ?? 0),
      debt_amount: Number(r.debt_amount ?? 0),
    })),
  );
});
