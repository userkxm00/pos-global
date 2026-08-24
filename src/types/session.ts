// Local POS Session domain types.
// F1.05 — Local user/session model

export interface LocalSession {
  id: string
  user_id: string
  branch_id: string
  auth_level: string
  created_at: string
  expires_at: string
  revoked_at?: string | null
}

export interface SessionContext {
  session_id: string
  user_id: string
  full_name: string
  username?: string | null
  role: string
  branch_id: string
  organization_id?: string | null
  auth_level: string
  expires_at: string
}

export interface LoginResult {
  success: boolean
  session_id?: string | null
  user_id?: string | null
  role?: string | null
  branch_id?: string | null
}

export interface AuthState {
  authenticated: boolean
  session_id?: string | null
  user_id?: string | null
  branch_id?: string | null
  role?: string | null
  organization_id?: string | null
}
