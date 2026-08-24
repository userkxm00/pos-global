// Online identity and Supabase Auth contracts.
// F1.04 — Supabase Auth adapter

export interface CloudUser {
  id: string
  email: string
  createdAt?: string
  lastSignInAt?: string
}

export interface CloudSession {
  accessToken: string
  refreshToken?: string
  expiresAt?: number
  expiresIn?: number
  tokenType?: string
  user: CloudUser
}

export interface SignInCredentials {
  email: string
  password: string
}

export type AuthErrorCode =
  | 'INVALID_CREDENTIALS'
  | 'NETWORK_ERROR'
  | 'UNCONFIGURED'
  | 'SESSION_EXPIRED'
  | 'INVALID_RESPONSE'
  | 'VALIDATION_ERROR'
  | 'UNKNOWN'

export interface AuthErrorDetails {
  code: AuthErrorCode
  message: string
  status?: number
}

export interface SupabaseAuthConfig {
  url: string
  publishableKey: string
}

export interface AuthAdapter {
  signIn(credentials: SignInCredentials): Promise<CloudSession>
  signOut(): Promise<void>
  getSession(): Promise<CloudSession | null>
  refreshSession(): Promise<CloudSession | null>
  getUser(): Promise<CloudUser | null>
}
