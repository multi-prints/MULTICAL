import { Hono } from "hono";
import { turso } from "../db";
import type { Env } from "../env";

type AppEnv = { Bindings: Env };

/**
 * Admin business revenue statement aggregates (JSON).
 * Desktop clients turn this into a PDF; multi-PC path can read the same figures.
 *
 * Query:
 *   source = sales | printing | both
 *   months = 1..6  (rolling window counted backward from today)
 */
export const statements = new Hono<AppEnv>();

function methodLabel(m: string): string {
  return String(m ?? "cash").toLowerCase();
}

statements.get("/", async (c) => {
  const sourceRaw = (c.req.query("source") ?? "both").trim().toLowerCase();
  const months = Math.min(6, Math.max(1, Number(c.req.query("months") ?? 3) || 3));
  const requested_by = c.req.query("requested_by") ?? null;

  if (!["sales", "printing", "both"].includes(sourceRaw)) {
    return c.json({ error: "source must be 'sales', 'printing', or 'both'" }, 400);
  }

  const includeSales = sourceRaw === "sales" || sourceRaw === "both";
  const includePrinting = sourceRaw === "printing" || sourceRaw === "both";
  const db = turso(c.env);

  const bounds = await db.execute({
    sql: `SELECT DATE('now', 'localtime') AS period_end,
                 DATE('now', 'localtime', ?) AS period_start`,
    args: [`-${months} months`],
  });
  const period_end = String(bounds.rows[0]?.period_end ?? "");
  const period_start = String(bounds.rows[0]?.period_start ?? "");

  let sales = null as null | Record<string, unknown>;
  if (includeSales) {
    const sum = await db.execute({
      sql: `SELECT
              COUNT(*) AS transaction_count,
              COALESCE(SUM(amount), 0) AS gross_billed,
              COALESCE(SUM(CASE WHEN COALESCE(is_debt, 0) = 0 THEN amount ELSE 0 END), 0) AS cash_collected,
              COALESCE(SUM(CASE WHEN COALESCE(is_debt, 0) > 0 THEN 1 ELSE 0 END), 0) AS debt_transactions,
              COALESCE(SUM(CASE WHEN COALESCE(is_debt, 0) > 0 THEN amount ELSE 0 END), 0) AS debt_billed
            FROM sales
            WHERE DATE(timestamp) >= DATE(?) AND DATE(timestamp) <= DATE(?)`,
      args: [period_start, period_end],
    });
    const debtPay = await db.execute({
      sql: `SELECT COALESCE(SUM(dp.amount), 0) AS amt
            FROM debt_payments dp
            INNER JOIN debts d ON d.id = dp.debt_id
            WHERE d.sale_id IS NOT NULL
              AND DATE(dp.payment_date) >= DATE(?)
              AND DATE(dp.payment_date) <= DATE(?)`,
      args: [period_start, period_end],
    });
    const counts = await db.execute({
      sql: `SELECT
              COALESCE(SUM(CASE WHEN type = 'product' THEN 1 ELSE 0 END), 0) AS product_sales_count,
              COALESCE(SUM(CASE WHEN type = 'stock' THEN 1 ELSE 0 END), 0) AS stock_sales_count
            FROM sales
            WHERE DATE(timestamp) >= DATE(?) AND DATE(timestamp) <= DATE(?)`,
      args: [period_start, period_end],
    });
    const pm = await db.execute({
      sql: `SELECT LOWER(COALESCE(payment_method, 'cash')) AS method,
                   COUNT(*) AS cnt,
                   COALESCE(SUM(amount), 0) AS amount
            FROM sales
            WHERE DATE(timestamp) >= DATE(?) AND DATE(timestamp) <= DATE(?)
              AND COALESCE(is_debt, 0) = 0
            GROUP BY LOWER(COALESCE(payment_method, 'cash'))
            ORDER BY amount DESC`,
      args: [period_start, period_end],
    });
    const r = sum.rows[0] ?? {};
    sales = {
      transaction_count: Number(r.transaction_count ?? 0),
      gross_billed: Number(r.gross_billed ?? 0),
      cash_collected:
        Number(r.cash_collected ?? 0) + Number(debtPay.rows[0]?.amt ?? 0),
      debt_transactions: Number(r.debt_transactions ?? 0),
      debt_billed: Number(r.debt_billed ?? 0),
      product_sales_count: Number(counts.rows[0]?.product_sales_count ?? 0),
      stock_sales_count: Number(counts.rows[0]?.stock_sales_count ?? 0),
      payment_methods: pm.rows.map((row) => ({
        method: methodLabel(String(row.method ?? "cash")),
        count: Number(row.cnt ?? 0),
        amount: Number(row.amount ?? 0),
      })),
    };
  }

  let printing = null as null | Record<string, unknown>;
  if (includePrinting) {
    const sum = await db.execute({
      sql: `SELECT
              COUNT(*) AS job_count,
              COALESCE(SUM(amount), 0) AS gross_billed,
              COALESCE(SUM(CASE WHEN COALESCE(is_debt, 0) = 0 THEN amount ELSE 0 END), 0) AS cash_collected,
              COALESCE(SUM(CASE WHEN COALESCE(is_debt, 0) > 0 THEN 1 ELSE 0 END), 0) AS debt_jobs,
              COALESCE(SUM(CASE WHEN COALESCE(is_debt, 0) > 0 THEN amount ELSE 0 END), 0) AS debt_billed,
              COALESCE(SUM(COALESCE(stock_metres_used, 0)), 0) AS material_metres_used
            FROM service_transactions
            WHERE DATE(timestamp) >= DATE(?) AND DATE(timestamp) <= DATE(?)`,
      args: [period_start, period_end],
    });
    const debtPay = await db.execute({
      sql: `SELECT COALESCE(SUM(dp.amount), 0) AS amt
            FROM debt_payments dp
            INNER JOIN debts d ON d.id = dp.debt_id
            WHERE d.service_transaction_id IS NOT NULL
              AND DATE(dp.payment_date) >= DATE(?)
              AND DATE(dp.payment_date) <= DATE(?)`,
      args: [period_start, period_end],
    });
    const pm = await db.execute({
      sql: `SELECT LOWER(COALESCE(payment_method, 'cash')) AS method,
                   COUNT(*) AS cnt,
                   COALESCE(SUM(amount), 0) AS amount
            FROM service_transactions
            WHERE DATE(timestamp) >= DATE(?) AND DATE(timestamp) <= DATE(?)
              AND COALESCE(is_debt, 0) = 0
            GROUP BY LOWER(COALESCE(payment_method, 'cash'))
            ORDER BY amount DESC`,
      args: [period_start, period_end],
    });
    const r = sum.rows[0] ?? {};
    printing = {
      job_count: Number(r.job_count ?? 0),
      gross_billed: Number(r.gross_billed ?? 0),
      cash_collected:
        Number(r.cash_collected ?? 0) + Number(debtPay.rows[0]?.amt ?? 0),
      debt_jobs: Number(r.debt_jobs ?? 0),
      debt_billed: Number(r.debt_billed ?? 0),
      material_metres_used: Number(r.material_metres_used ?? 0),
      payment_methods: pm.rows.map((row) => ({
        method: methodLabel(String(row.method ?? "cash")),
        count: Number(row.cnt ?? 0),
        amount: Number(row.amount ?? 0),
      })),
    };
  }

  // Outstanding receivables for debts created in period
  let outstandingSql = `SELECT COALESCE(SUM(remaining_amount), 0) AS amt
    FROM debts
    WHERE status = 'pending'
      AND DATE(created_at) >= DATE(?)
      AND DATE(created_at) <= DATE(?)`;
  if (includeSales && !includePrinting) {
    outstandingSql += ` AND sale_id IS NOT NULL`;
  } else if (includePrinting && !includeSales) {
    outstandingSql += ` AND service_transaction_id IS NOT NULL`;
  }
  const outstanding = await db.execute({
    sql: outstandingSql,
    args: [period_start, period_end],
  });

  // Monthly breakdown across calendar months in the window
  const monthlyRes = await db.execute({
    sql: `WITH RECURSIVE months(idx, month_start) AS (
            SELECT 0, DATE(?, 'start of month')
            UNION ALL
            SELECT idx + 1, DATE(month_start, '+1 month')
            FROM months
            WHERE DATE(month_start, '+1 month') <= DATE(?, 'start of month')
              AND idx < 12
          )
          SELECT strftime('%Y-%m', month_start) AS ym,
                 CASE strftime('%m', month_start)
                   WHEN '01' THEN 'Jan' WHEN '02' THEN 'Feb' WHEN '03' THEN 'Mar'
                   WHEN '04' THEN 'Apr' WHEN '05' THEN 'May' WHEN '06' THEN 'Jun'
                   WHEN '07' THEN 'Jul' WHEN '08' THEN 'Aug' WHEN '09' THEN 'Sep'
                   WHEN '10' THEN 'Oct' WHEN '11' THEN 'Nov' WHEN '12' THEN 'Dec'
                 END || ' ' || strftime('%Y', month_start) AS label,
                 month_start
          FROM months
          ORDER BY idx ASC`,
    args: [period_start, period_end],
  });

  const monthly = [];
  for (const m of monthlyRes.rows) {
    const ym = String(m.ym ?? "");
    let sales_revenue = 0;
    let sales_count = 0;
    let printing_revenue = 0;
    let printing_count = 0;

    if (includeSales) {
      const s = await db.execute({
        sql: `SELECT
                COALESCE(SUM(CASE WHEN COALESCE(is_debt, 0) = 0 THEN amount ELSE 0 END), 0) AS rev,
                COUNT(*) AS cnt
              FROM sales
              WHERE strftime('%Y-%m', timestamp) = ?
                AND DATE(timestamp) >= DATE(?)
                AND DATE(timestamp) <= DATE(?)`,
        args: [ym, period_start, period_end],
      });
      sales_revenue = Number(s.rows[0]?.rev ?? 0);
      sales_count = Number(s.rows[0]?.cnt ?? 0);
    }
    if (includePrinting) {
      const p = await db.execute({
        sql: `SELECT
                COALESCE(SUM(CASE WHEN COALESCE(is_debt, 0) = 0 THEN amount ELSE 0 END), 0) AS rev,
                COUNT(*) AS cnt
              FROM service_transactions
              WHERE strftime('%Y-%m', timestamp) = ?
                AND DATE(timestamp) >= DATE(?)
                AND DATE(timestamp) <= DATE(?)`,
        args: [ym, period_start, period_end],
      });
      printing_revenue = Number(p.rows[0]?.rev ?? 0);
      printing_count = Number(p.rows[0]?.cnt ?? 0);
    }

    monthly.push({
      year_month: ym,
      label: String(m.label ?? ym),
      sales_revenue,
      sales_count,
      printing_revenue,
      printing_count,
      total_revenue: sales_revenue + printing_revenue,
      total_count: sales_count + printing_count,
    });
  }

  const total_gross_billed =
    Number(sales?.gross_billed ?? 0) + Number(printing?.gross_billed ?? 0);
  const total_cash_collected =
    Number(sales?.cash_collected ?? 0) + Number(printing?.cash_collected ?? 0);
  const total_transactions =
    Number(sales?.transaction_count ?? 0) + Number(printing?.job_count ?? 0);

  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  const generated_at = `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())} ${pad(now.getHours())}:${pad(now.getMinutes())}`;

  return c.json({
    source: sourceRaw,
    months,
    period_start,
    period_end,
    generated_at,
    requested_by,
    app_version: "api",
    total_gross_billed,
    total_cash_collected,
    total_transactions,
    average_monthly_cash: months > 0 ? total_cash_collected / months : 0,
    period_outstanding_receivables: Number(outstanding.rows[0]?.amt ?? 0),
    sales,
    printing,
    monthly,
  });
});
