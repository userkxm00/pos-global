// Constants for Authentication Screens & Session Lifecycle
// F1.13 — Authentication screens and session lifecycle & F1.19 — Supabase Auth adapter hardening

export const AUTH_STORAGE_KEYS = {
  SESSION_ID: 'pos_global_session_id',
  AUTH_MODE: 'pos_global_auth_mode',
  CLOUD_TOKEN: 'pos_global_cloud_token',
  CLOUD_REFRESH_TOKEN: 'pos_global_cloud_refresh_token',
  CLOUD_EXPIRES_AT: 'pos_global_cloud_expires_at',
  CLOUD_USER: 'pos_global_cloud_user',
} as const

export const DEFAULT_AUTH_MODE = 'online' as const
