export type Env = {
  TURSO_DATABASE_URL: string;
  TURSO_AUTH_TOKEN: string;
  /** Shared secret desktop apps send as Authorization: Bearer <secret> */
  API_SECRET: string;
  APP_NAME?: string;
};
