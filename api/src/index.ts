import { Hono } from "hono";
import { cors } from "hono/cors";
import { requireApiSecret } from "./auth";
import type { Env } from "./env";
import { turso } from "./db";
import { materials } from "./routes/materials";
import { printing } from "./routes/printing";
import { products } from "./routes/products";
import { sales } from "./routes/sales";
import { stock } from "./routes/stock";

type AppEnv = { Bindings: Env };

const app = new Hono<AppEnv>();

app.use(
  "*",
  cors({
    origin: "*",
    allowMethods: ["GET", "POST", "PATCH", "DELETE", "OPTIONS"],
    allowHeaders: ["Content-Type", "Authorization"],
  }),
);

app.get("/", (c) =>
  c.json({
    name: c.env.APP_NAME ?? "MULTIPRINTS API",
    status: "ok",
    docs: "See api/README.md — set TURSO_* and API_SECRET secrets, then deploy.",
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

// Everything below requires Authorization: Bearer <API_SECRET>
app.use("/v1/*", requireApiSecret);

app.route("/v1/products", products);
app.route("/v1/stock", stock);
app.route("/v1/materials", materials);
app.route("/v1/sales", sales);
app.route("/v1/printing", printing);

app.notFound((c) => c.json({ error: "Not found" }, 404));
app.onError((err, c) => {
  console.error(err);
  return c.json(
    { error: err instanceof Error ? err.message : "Internal error" },
    500,
  );
});

export default app;
