// TypeScript interfaces for Register domain model.
// Matches Rust domain model in src-tauri/src/register/mod.rs

export interface Register {
  id: string
  organization_id: string
  branch_id: string
  name: string
  code: string | null
  is_active: boolean
  created_at: string
}

export interface CreateRegisterInput {
  organization_id: string
  branch_id: string
  name: string
  code?: string | null
  is_active?: boolean
}

export interface UpdateRegisterInput {
  id: string
  name: string
  code?: string | null
  is_active: boolean
}
