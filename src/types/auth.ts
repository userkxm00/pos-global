// Online identity and Supabase Auth contracts.
// F1.04 — Supabase Auth adapter & F1.13 — Authentication screens and session lifecycle

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

export interface LocalSignInInput {
  username: string
  password: string
}

export type AuthMode = 'online' | 'local'
export type AuthStatus = 'unauthenticated' | 'authenticating' | 'authenticated' | 'expired'
