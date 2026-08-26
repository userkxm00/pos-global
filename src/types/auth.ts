// F1.04 — Supabase Auth adapter & F1.13 — Auth screens & F1.14 — Local PIN and Lock screen & F1.19 — Supabase Auth adapter hardening

export interface SupabaseAuthConfig {
  url: string
  publishable_key: string
}

export interface OnlineIdentity {
  id: string
  email: string
  created_at?: string
  last_sign_in_at?: string
}

export interface OnlineSession {
  access_token: string
  refresh_token?: string
  expires_at?: number
  expires_in?: number
  token_type?: string
  user: OnlineIdentity
}

export interface SignInInput {
  email: string
  password: string
}

export interface RefreshTokenInput {
  refresh_token: string
}

export interface LocalSignInInput {
  username: string
  password: string
}

export type AuthMode = 'online' | 'local'
export type AuthStatus = 'unauthenticated' | 'authenticating' | 'authenticated' | 'expired' | 'locked'

export type AuthErrorCode =
  | 'invalid_credentials'
  | 'session_expired'
  | 'rate_limit'
  | 'network_error'
  | 'service_unavailable'
  | 'unconfigured'
  | 'validation_error'
  | 'security_violation'
  | 'unknown'

export interface TypedAuthError {
  code: AuthErrorCode
  message: string
}
