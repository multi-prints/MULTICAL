import { createClient, type Client } from "@libsql/client/web";
import type { Env } from "./env";

export function turso(env: Env): Client {
  if (!env.TURSO_DATABASE_URL || !env.TURSO_AUTH_TOKEN) {
    throw new Error(
      "Missing TURSO_DATABASE_URL or TURSO_AUTH_TOKEN secrets on the Worker",
    );
  }
  return createClient({
    url: env.TURSO_DATABASE_URL,
    authToken: env.TURSO_AUTH_TOKEN,
  });
}

/**
 * Collision-resistant positive id (JS-safe ≤ 2^53-1).
 * Layout matches desktop intent: time bits + small random.
 */
export function newDistributedId(): number {
  const ms = Date.now(); // ~2^41 until year ~2100
  const rand = Math.floor(Math.random() * 1024); // 10 bits
  // ms << 10 | rand  stays well under Number.MAX_SAFE_INTEGER for decades
  return ms * 1024 + rand;
}

export function productNaturalKey(
  productType: string,
  color?: string | null,
  size?: string | null,
): string {
  return [
    productType.trim().toLowerCase(),
    (color ?? "").trim().toLowerCase(),
    (size ?? "").trim().toLowerCase(),
  ].join("|");
}

export function stockNaturalKey(
  color: string,
  size: string,
  stickerType: string,
): string {
  return [
    color.trim().toLowerCase(),
    size.trim().toLowerCase(),
    stickerType.trim().toLowerCase(),
  ].join("|");
}

/**
 * Parse entity ids from JSON (string or number). Desktop sends i64 as strings.
 */
export function asId(v: unknown): number | null {
  if (v == null || v === "") return null;
  if (typeof v === "number" && Number.isFinite(v)) return Math.trunc(v);
  if (typeof v === "bigint") {
    const n = Number(v);
    return Number.isFinite(n) ? Math.trunc(n) : null;
  }
  if (typeof v === "string" && v.trim()) {
    const n = Number(v.trim());
    if (Number.isFinite(n)) return Math.trunc(n);
  }
  return null;
}

export function asInt(v: unknown, fallback = 0): number {
  if (typeof v === "boolean") return v ? 1 : 0;
  if (typeof v === "number" && Number.isFinite(v)) return Math.trunc(v);
  if (typeof v === "string" && v.trim()) {
    const n = Number(v.trim());
    if (Number.isFinite(n)) return Math.trunc(n);
  }
  return fallback;
}

let createdByColumnsReady = false;

/**
 * Ensure sales / service_transactions have created_by (staff username).
 * Safe to call repeatedly — duplicate-column errors are ignored.
 */
export async function ensureCreatedByColumns(db: Client): Promise<void> {
  if (createdByColumnsReady) return;
  for (const sql of [
    "ALTER TABLE sales ADD COLUMN created_by TEXT",
    "ALTER TABLE service_transactions ADD COLUMN created_by TEXT",
  ]) {
    try {
      await db.execute(sql);
    } catch {
      // column already exists
    }
  }
  createdByColumnsReady = true;
}
