// TypeScript interfaces for Organization domain model.
// Matches Rust domain model in src-tauri/src/organization/mod.rs

export interface Organization {
  id: string
  name: string
  default_currency: string
  default_language: string
  created_at: string
}

export interface CreateOrganizationInput {
  name: string
  default_currency?: string
  default_language?: string
}

export interface UpdateOrganizationInput {
  id: string
  name: string
  default_currency: string
  default_language: string
}
