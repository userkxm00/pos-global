// Constants for Authentication Screens & Session Lifecycle
// F1.13 — Authentication screens and session lifecycle

export const AUTH_STORAGE_KEYS = {
  SESSION_ID: 'pos_global_session_id',
  AUTH_MODE: 'pos_global_auth_mode',
  CLOUD_TOKEN: 'pos_global_cloud_token',
  CLOUD_USER: 'pos_global_cloud_user',
} as const

export const DEFAULT_AUTH_MODE = 'online' as const
