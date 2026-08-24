import { createClient, type SupabaseClient } from '@supabase/supabase-js'
import type { SupabaseAuthConfig } from '../types/auth'

let defaultClient: SupabaseClient | null = null

export function getSupabaseConfig(): SupabaseAuthConfig | null {
  const url = import.meta.env.VITE_SUPABASE_URL as string | undefined
  const key = import.meta.env.VITE_SUPABASE_PUBLISHABLE_KEY as string | undefined

  if (url && key && url.trim() && key.trim()) {
    return {
      url: url.trim(),
      publishableKey: key.trim(),
    }
  }
  return null
}

export function createSupabaseClient(config: SupabaseAuthConfig): SupabaseClient {
  return createClient(config.url, config.publishableKey, {
    auth: {
      persistSession: true,
      autoRefreshToken: true,
      detectSessionInUrl: false,
    },
  })
}

export function getSupabaseClient(config?: SupabaseAuthConfig): SupabaseClient | null {
  if (config) {
    return createSupabaseClient(config)
  }

  if (!defaultClient) {
    const envConfig = getSupabaseConfig()
    if (envConfig) {
      defaultClient = createSupabaseClient(envConfig)
    }
  }

  return defaultClient
}

// Default export: initialized if environment configuration exists
const envConfig = getSupabaseConfig()
export const supabase = envConfig ? createSupabaseClient(envConfig) : null
