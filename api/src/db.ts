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
