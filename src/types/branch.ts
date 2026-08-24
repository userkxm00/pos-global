// TypeScript interfaces for Branch domain model.
// Matches Rust domain model in src-tauri/src/branch/mod.rs

export interface Branch {
  id: string
  organization_id: string
  name: string
  address: string | null
  currency: string
  is_active: boolean
  created_at: string
}

export interface CreateBranchInput {
  organization_id: string
  name: string
  address?: string | null
  currency?: string
  is_active?: boolean
}

export interface UpdateBranchInput {
  id: string
  name: string
  address?: string | null
  currency: string
  is_active: boolean
}
