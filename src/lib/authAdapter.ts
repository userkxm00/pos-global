// Supabase Auth adapter implementation for online account identity.
// F1.04 — Supabase Auth adapter

import type { SupabaseClient, User, Session, AuthError as SupabaseAuthError } from '@supabase/supabase-js'
import type {
  AuthAdapter,
  AuthErrorCode,
  AuthErrorDetails,
  CloudSession,
  CloudUser,
  SignInCredentials,
  SupabaseAuthConfig,
} from '../types/auth'
import { getSupabaseClient } from './supabase'

export class AuthAdapterError extends Error {
  readonly code: AuthErrorCode
  readonly status?: number

  constructor(details: AuthErrorDetails) {
    super(details.message)
    this.name = 'AuthAdapterError'
    this.code = details.code
    this.status = details.status
    Object.setPrototypeOf(this, AuthAdapterError.prototype)
  }
}

export function mapSupabaseUser(user: User): CloudUser {
  return {
    id: user.id,
    email: user.email || '',
    createdAt: user.created_at,
    lastSignInAt: user.last_sign_in_at,
  }
}

export function mapSupabaseSession(session: Session): CloudSession {
  return {
    accessToken: session.access_token,
    refreshToken: session.refresh_token,
    expiresAt: session.expires_at,
    expiresIn: session.expires_in,
    tokenType: session.token_type,
    user: mapSupabaseUser(session.user),
  }
}

export function mapAuthError(error: unknown): AuthAdapterError {
  if (error instanceof AuthAdapterError) {
    return error
  }

  if (typeof error === 'object' && error !== null) {
    const sbError = error as Partial<SupabaseAuthError>
    const message = sbError.message || 'Authentication operation failed'
    const status = sbError.status

    if (
      message.toLowerCase().includes('invalid login credentials') ||
      message.toLowerCase().includes('invalid grant') ||
      message.toLowerCase().includes('invalid credentials') ||
      status === 400
    ) {
      return new AuthAdapterError({
        code: 'INVALID_CREDENTIALS',
        message: 'Invalid email or password',
        status,
      })
    }

    if (
      message.toLowerCase().includes('jwt expired') ||
      message.toLowerCase().includes('session expired') ||
      status === 401
    ) {
      return new AuthAdapterError({
        code: 'SESSION_EXPIRED',
        message: 'Session has expired. Please sign in again.',
        status,
      })
    }

    if (
      message.toLowerCase().includes('failed to fetch') ||
      message.toLowerCase().includes('network') ||
      message.toLowerCase().includes('timeout') ||
      message.toLowerCase().includes('offline')
    ) {
      return new AuthAdapterError({
        code: 'NETWORK_ERROR',
        message: 'Unable to reach authentication service. Please check your network connection.',
        status,
      })
    }

    return new AuthAdapterError({
      code: 'UNKNOWN',
      message: 'An error occurred during authentication',
      status,
    })
  }

  return new AuthAdapterError({
    code: 'UNKNOWN',
    message: 'An unexpected authentication error occurred',
  })
}

export class SupabaseAuthAdapter implements AuthAdapter {
  private client: SupabaseClient | null

  constructor(clientOrConfig?: SupabaseClient | SupabaseAuthConfig) {
    if (!clientOrConfig) {
      this.client = getSupabaseClient()
    } else if ('auth' in clientOrConfig && typeof clientOrConfig.auth === 'object') {
      this.client = clientOrConfig as SupabaseClient
    } else {
      const config = clientOrConfig as SupabaseAuthConfig
      this.client = getSupabaseClient(config)
    }
  }

  private ensureClient(): SupabaseClient {
    if (!this.client) {
      throw new AuthAdapterError({
        code: 'UNCONFIGURED',
        message: 'Supabase authentication is not configured with valid public URL and publishable key',
      })
    }
    return this.client
  }

  async signIn(credentials: SignInCredentials): Promise<CloudSession> {
    const client = this.ensureClient()

    if (!credentials.email || !credentials.email.trim()) {
      throw new AuthAdapterError({
        code: 'VALIDATION_ERROR',
        message: 'Email address cannot be empty',
      })
    }

    if (!credentials.password) {
      throw new AuthAdapterError({
        code: 'VALIDATION_ERROR',
        message: 'Password cannot be empty',
      })
    }

    try {
      const { data, error } = await client.auth.signInWithPassword({
        email: credentials.email.trim(),
        password: credentials.password,
      })

      if (error) {
        throw mapAuthError(error)
      }

      if (!data.session || !data.user) {
        throw new AuthAdapterError({
          code: 'INVALID_RESPONSE',
          message: 'Supabase returned an incomplete authentication response',
        })
      }

      return mapSupabaseSession(data.session)
    } catch (err) {
      throw mapAuthError(err)
    }
  }

  async signOut(): Promise<void> {
    const client = this.ensureClient()

    try {
      const { error } = await client.auth.signOut()
      if (error) {
        throw mapAuthError(error)
      }
    } catch (err) {
      throw mapAuthError(err)
    }
  }

  async getSession(): Promise<CloudSession | null> {
    const client = this.ensureClient()

    try {
      const { data, error } = await client.auth.getSession()
      if (error) {
        throw mapAuthError(error)
      }

      return data.session ? mapSupabaseSession(data.session) : null
    } catch (err) {
      throw mapAuthError(err)
    }
  }

  async refreshSession(): Promise<CloudSession | null> {
    const client = this.ensureClient()

    try {
      const { data, error } = await client.auth.refreshSession()
      if (error) {
        throw mapAuthError(error)
      }

      return data.session ? mapSupabaseSession(data.session) : null
    } catch (err) {
      throw mapAuthError(err)
    }
  }

  async getUser(): Promise<CloudUser | null> {
    const client = this.ensureClient()

    try {
      const { data, error } = await client.auth.getUser()
      if (error) {
        throw mapAuthError(error)
      }

      return data.user ? mapSupabaseUser(data.user) : null
    } catch (err) {
      throw mapAuthError(err)
    }
  }
}

export function createSupabaseAuthAdapter(
  clientOrConfig?: SupabaseClient | SupabaseAuthConfig
): SupabaseAuthAdapter {
  return new SupabaseAuthAdapter(clientOrConfig)
}
