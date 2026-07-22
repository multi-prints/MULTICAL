import { Hono } from "hono";
import { cors } from "hono/cors";
import { requireApiSecret } from "./auth";
import type { Env } from "./env";
import { turso } from "./db";
import { dashboard } from "./routes/dashboard";
import { debts } from "./routes/debts";
import { materials } from "./routes/materials";
import { notifications } from "./routes/notifications";
import { printing } from "./routes/printing";
import { products } from "./routes/products";
import { sales } from "./routes/sales";
import { services } from "./routes/services";
import { stock } from "./routes/stock";
import { auth, users } from "./routes/users";

type AppEnv = { Bindings: Env };

const app = new Hono<AppEnv>();

app.use(
  "*",
  cors({
    origin: "*",
    allowMethods: ["GET", "POST", "PATCH", "DELETE", "OPTIONS"],
    allowHeaders: ["Content-Type", "Authorization", "X-Session-Token"],
  }),
);

app.get("/", (c) =>
  c.json({
    name: c.env.APP_NAME ?? "MULTIPRINTS API",
    status: "ok",
    coverage:
      "products, stock, materials, sales, printing, debts, notifications, dashboard, users, auth, services",
  }),
);

app.get("/health", async (c) => {
  try {
    const db = turso(c.env);
    const r = await db.execute("SELECT 1 AS ok");
    return c.json({
      ok: true,
      turso: Number(r.rows[0]?.ok) === 1,
      app: c.env.APP_NAME ?? "MULTIPRINTS API",
    });
  } catch (e) {
    return c.json(
      {
        ok: false,
        turso: false,
        error: e instanceof Error ? e.message : String(e),
      },
      500,
    );
  }
});

// Shop API secret required for all /v1/*
app.use("/v1/*", requireApiSecret);

app.route("/v1/products", products);
app.route("/v1/stock", stock);
app.route("/v1/materials", materials);
app.route("/v1/sales", sales);
app.route("/v1/printing", printing);
app.route("/v1/debts", debts);
app.route("/v1/notifications", notifications);
app.route("/v1/dashboard", dashboard);
app.route("/v1/users", users);
app.route("/v1/auth", auth);
app.route("/v1/services", services);

app.notFound((c) => c.json({ error: "Not found" }, 404));
app.onError((err, c) => {
  console.error(err);
  return c.json(
    { error: err instanceof Error ? err.message : "Internal error" },
    500,
  );
});

export default app;
