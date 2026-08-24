// Local POS User domain types.
// F1.05 — Local user/session model

export interface User {
  id: string
  branch_id: string
  full_name: string
  username?: string | null
  role: string
  is_active: boolean
  supabase_user_id?: string | null
  auth_provider: string
  created_at: string
}

export interface CreateUserInput {
  branch_id: string
  full_name: string
  username?: string | null
  password?: string | null
  pin?: string | null
  role: string
  supabase_user_id?: string | null
  auth_provider?: string
}

export interface UpdateUserInput {
  full_name?: string | null
  username?: string | null
  password?: string | null
  pin?: string | null
  role?: string | null
  is_active?: boolean | null
  supabase_user_id?: string | null
}
