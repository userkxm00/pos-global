// Authoritative Tauri IPC & Supabase Auth Client for Authentication and Session Lifecycle
// Invokes production Tauri commands in src-tauri/src/commands/auth.rs and cloud auth revocation
// F1.04 — Supabase Auth adapter & F1.13 — Auth screens & F1.14 — Local PIN and Lock screen & F1.19 — Supabase Auth adapter hardening

import type {
  SupabaseAuthConfig,
  OnlineSession,
  SignInInput,
  LocalSignInInput,
  AuthErrorCode,
  TypedAuthError,
} from '../types/auth'
import type { AuthState, LoginResult } from '../types/session'

export interface AuthApiClient {
  onlineLogin(credentials: SignInInput, config?: SupabaseAuthConfig): Promise<OnlineSession>
  refreshOnlineSession(refreshToken: string, config?: SupabaseAuthConfig): Promise<OnlineSession>
  onlineLogout(token?: string | null, config?: SupabaseAuthConfig): Promise<void>
  localLogin(credentials: LocalSignInInput): Promise<LoginResult>
  verifyPin(userId: string, pin: string): Promise<LoginResult>
  getAuthState(sessionId?: string | null): Promise<AuthState>
  logout(sessionId: string): Promise<void>
}

/**
 * Extracts a human-readable error message from unknown error objects.
 */
export function extractInvokeErrorMessage(err: unknown): string {
  if (typeof err === 'string') return err
  if (err instanceof Error) return err.message
  return String(err)
}

export function getDefaultSupabaseConfig(): SupabaseAuthConfig {
  return {
    url: 'https://mock.supabase.co',
    publishable_key: 'mock-anon-key',
  }
}

export function stripTrailingSlash(url: string): string {
  let end = url.length
  while (end > 0 && url.charCodeAt(end - 1) === 47) {
    end--
  }
  return url.slice(0, end)
}

export const VALID_AUTH_ERROR_CODES = new Set<AuthErrorCode>([
  'invalid_credentials',
  'session_expired',
  'rate_limit',
  'network_error',
  'service_unavailable',
  'unconfigured',
  'validation_error',
  'security_violation',
  'unknown',
])

export class AuthErrorInstance extends Error implements TypedAuthError {
  code: AuthErrorCode
  details?: Record<string, unknown>

  constructor(code: AuthErrorCode, message: string, details?: Record<string, unknown>) {
    super(message)
    this.name = 'TypedAuthError'
    this.code = code
    this.details = details
    Object.setPrototypeOf(this, AuthErrorInstance.prototype)
  }
}

export function createTypedAuthError(
  code: AuthErrorCode,
  message: string,
  details?: Record<string, unknown>,
): TypedAuthError {
  return new AuthErrorInstance(code, message, details)
}

const SESSION_EXPIRED_PATTERNS = [
  'session expired',
  'invalid refresh token',
  'refresh token is invalid',
  'refresh token not found',
  'refresh_token_not_found',
  'already used',
  'jwt expired',
  'session has expired',
]

const CREDENTIAL_PATTERNS = [
  'invalid credentials',
  'invalid login credentials',
  'invalid email',
  'invalid pin',
  'user not found',
  'does not match',
]

const NETWORK_PATTERNS = [
  'network',
  'unable to reach',
  'failed to fetch',
  'connection refused',
]

function extractErrorMessage(err: unknown): string {
  if (err instanceof Error) return err.message
  if (typeof err === 'string') return err
  if (err && typeof err === 'object') {
    const obj = err as Record<string, unknown>
    if (typeof obj.message === 'string') return obj.message
    if (typeof obj.error_description === 'string') return obj.error_description
    if (typeof obj.error === 'string') return obj.error
    try {
      return JSON.stringify(err)
    } catch {
      return 'Unknown error'
    }
  }
  return typeof err === 'number' || typeof err === 'boolean' ? String(err) : ''
}

function matchAuthErrorCode(lower: string): AuthErrorCode {
  if (SESSION_EXPIRED_PATTERNS.some((p) => lower.includes(p))) return 'session_expired'
  if (lower.includes('rate limit') || lower.includes('too many')) return 'rate_limit'
  if (NETWORK_PATTERNS.some((p) => lower.includes(p))) return 'network_error'
  if (CREDENTIAL_PATTERNS.some((p) => lower.includes(p))) return 'invalid_credentials'
  if (lower.includes('unavailable') || lower.includes('service unavailable')) return 'service_unavailable'
  if (lower.includes('security violation') || lower.includes('forbidden') || lower.includes('secret key')) return 'security_violation'
  if (lower.includes('unconfigured') || lower.includes('missing configuration')) return 'unconfigured'
  if (lower.includes('validation')) return 'validation_error'
  return 'unknown'
}

/**
 * Classifies an unknown error into a structured TypedAuthError.
 */
export function classifyAuthError(err: unknown): TypedAuthError {
  if (
    err &&
    typeof err === 'object' &&
    'code' in err &&
    typeof (err as { code: unknown }).code === 'string' &&
    VALID_AUTH_ERROR_CODES.has((err as { code: unknown }).code as AuthErrorCode) &&
    'message' in err &&
    typeof (err as { message: unknown }).message === 'string'
  ) {
    return err as TypedAuthError
  }

  const msg = extractErrorMessage(err)
  const code = matchAuthErrorCode(msg.toLowerCase())
  return createTypedAuthError(code, msg)
}

// Real Tauri IPC & Cloud Auth Implementation
class TauriAuthApiClient implements AuthApiClient {
  private async invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      return await invoke<T>(cmd, args)
    } catch (err) {
      throw classifyAuthError(err)
    }
  }

  async onlineLogin(credentials: SignInInput, config?: SupabaseAuthConfig): Promise<OnlineSession> {
    const activeConfig = config || getDefaultSupabaseConfig()
    return this.invoke<OnlineSession>('online_login', {
      config: activeConfig,
      credentials: {
        email: credentials.email.trim(),
        password: credentials.password,
      },
    })
  }

  async refreshOnlineSession(refreshToken: string, config?: SupabaseAuthConfig): Promise<OnlineSession> {
    const activeConfig = config || getDefaultSupabaseConfig()
    return this.invoke<OnlineSession>('refresh_online_session', {
      config: activeConfig,
      input: {
        refresh_token: refreshToken.trim(),
      },
    })
  }

  async onlineLogout(token?: string | null, config?: SupabaseAuthConfig): Promise<void> {
    if (!token) return
    const activeConfig = config || getDefaultSupabaseConfig()
    return this.invoke<void>('online_logout', {
      config: activeConfig,
      accessToken: token.trim(),
    })
  }

  async localLogin(credentials: LocalSignInInput): Promise<LoginResult> {
    return this.invoke<LoginResult>('login', {
      username: credentials.username.trim(),
      password: credentials.password,
    })
  }

  async verifyPin(userId: string, pin: string): Promise<LoginResult> {
    return this.invoke<LoginResult>('verify_pin', {
      userId: userId.trim(),
      pin: pin.trim(),
    })
  }

  async getAuthState(sessionId?: string | null): Promise<AuthState> {
    return this.invoke<AuthState>('auth_state', {
      sessionId: sessionId || null,
    })
  }

  async logout(sessionId: string): Promise<void> {
    return this.invoke<void>('logout', {
      sessionId,
    })
  }
}

let mockSessionCounter = 0

// In-Memory Mock Implementation for Deterministic Unit Tests & Local Dev Runs
export class MockAuthApiClient implements AuthApiClient {
  public shouldFailWith: string | null = null
  public isRateLimited: boolean = false
  public activeSessions: Map<string, AuthState> = new Map()
  public revokedCloudTokens: Set<string> = new Set()
  public refreshCount: number = 0
  public refreshDelayMs: number = 0

  async onlineLogin(credentials: SignInInput, _config?: SupabaseAuthConfig): Promise<OnlineSession> {
    if (this.shouldFailWith) throw classifyAuthError(this.shouldFailWith)
    const trimmedEmail = credentials.email.trim().toLowerCase()
    if (!trimmedEmail || !credentials.password) {
      throw classifyAuthError('Invalid credentials: Email and password are required')
    }
    if (credentials.password === 'wrong_password') {
      throw classifyAuthError('Invalid credentials: Invalid email or password')
    }

    const userId = `usr_cloud_${trimmedEmail.replace(/[^a-z0-9]/g, '_')}`
    return {
      access_token: `mock_jwt_token_${userId}`,
      refresh_token: `mock_refresh_${userId}`,
      expires_in: 3600,
      expires_at: Math.floor(Date.now() / 1000) + 3600,
      token_type: 'bearer',
      user: {
        id: userId,
        email: trimmedEmail,
        created_at: new Date().toISOString(),
        last_sign_in_at: new Date().toISOString(),
      },
    }
  }

  async refreshOnlineSession(refreshToken: string, _config?: SupabaseAuthConfig): Promise<OnlineSession> {
    this.refreshCount += 1
    if (this.refreshDelayMs > 0) {
      await new Promise((resolve) => setTimeout(resolve, this.refreshDelayMs))
    }
    if (this.shouldFailWith) throw classifyAuthError(this.shouldFailWith)
    const trimmedToken = refreshToken.trim()
    if (!trimmedToken) {
      throw classifyAuthError('Validation error: Refresh token cannot be empty')
    }
    if (trimmedToken === 'invalid_refresh' || trimmedToken === 'expired_refresh') {
      throw classifyAuthError('Session expired: Refresh token is invalid or has already been used. Please sign in again.')
    }
    if (trimmedToken === 'rate_limited') {
      throw classifyAuthError('Rate limit exceeded: Too many authentication requests. Please wait a moment and try again.')
    }
    if (trimmedToken === 'network_error') {
      throw classifyAuthError('Network error: Unable to reach Supabase authentication service')
    }
    if (trimmedToken === 'unknown_error') {
      throw classifyAuthError('Internal unclassified error')
    }

    const userId = 'usr_cloud_123'
    return {
      access_token: `mock_refreshed_jwt_${Date.now()}`,
      refresh_token: `mock_refreshed_refresh_${Date.now()}`,
      expires_in: 3600,
      expires_at: Math.floor(Date.now() / 1000) + 3600,
      token_type: 'bearer',
      user: {
        id: userId,
        email: 'owner@example.com',
        created_at: new Date().toISOString(),
        last_sign_in_at: new Date().toISOString(),
      },
    }
  }

  async onlineLogout(token?: string | null, _config?: SupabaseAuthConfig): Promise<void> {
    if (this.shouldFailWith) throw classifyAuthError(this.shouldFailWith)
    if (token) {
      this.revokedCloudTokens.add(token)
    }
  }

  async localLogin(credentials: LocalSignInInput): Promise<LoginResult> {
    if (this.shouldFailWith) throw classifyAuthError(this.shouldFailWith)
    const trimmedUsername = credentials.username.trim()
    if (!trimmedUsername || !credentials.password) {
      throw classifyAuthError('Invalid credentials: Username and password are required')
    }
    if (credentials.password === 'wrong_password') {
      throw classifyAuthError('Invalid credentials: Username or password does not match')
    }

    mockSessionCounter += 1
    const sessionId = `sess_${trimmedUsername.toLowerCase()}_${mockSessionCounter}`
    const userId = `usr_${trimmedUsername.toLowerCase()}`
    const authState: AuthState = {
      authenticated: true,
      session_id: sessionId,
      user_id: userId,
      branch_id: 'br_default',
      role: 'admin',
      organization_id: 'org_default',
    }
    this.activeSessions.set(sessionId, authState)

    return {
      success: true,
      session_id: sessionId,
      user_id: userId,
      role: 'admin',
      branch_id: 'br_default',
    }
  }

  async verifyPin(userId: string, pin: string): Promise<LoginResult> {
    if (this.shouldFailWith) throw classifyAuthError(this.shouldFailWith)
    const trimmedUserId = userId.trim()
    const trimmedPin = pin.trim()
    if (!trimmedUserId || !trimmedPin) {
      throw classifyAuthError('Validation error: User ID and PIN are required')
    }
    if (trimmedPin === '0000' || trimmedPin === 'wrong_pin') {
      throw classifyAuthError('Invalid credentials: Invalid PIN')
    }
    if (trimmedPin === '9999' || this.isRateLimited) {
      throw classifyAuthError(
        'Invalid credentials: Too many failed attempts. Account is temporarily locked. Please try again later.',
      )
    }

    mockSessionCounter += 1
    const sessionId = `sess_pin_${trimmedUserId.toLowerCase()}_${mockSessionCounter}`
    const authState: AuthState = {
      authenticated: true,
      session_id: sessionId,
      user_id: trimmedUserId,
      branch_id: 'br_default',
      role: 'cashier',
      organization_id: 'org_default',
    }
    this.activeSessions.set(sessionId, authState)

    return {
      success: true,
      session_id: sessionId,
      user_id: trimmedUserId,
      role: 'cashier',
      branch_id: 'br_default',
    }
  }

  async getAuthState(sessionId?: string | null): Promise<AuthState> {
    if (this.shouldFailWith) throw classifyAuthError(this.shouldFailWith)
    if (!sessionId) {
      return {
        authenticated: false,
        session_id: null,
        user_id: null,
        branch_id: null,
        role: null,
        organization_id: null,
      }
    }

    const session = this.activeSessions.get(sessionId)
    if (!session) {
      return {
        authenticated: false,
        session_id: null,
        user_id: null,
        branch_id: null,
        role: null,
        organization_id: null,
      }
    }

    return { ...session }
  }

  async logout(sessionId: string): Promise<void> {
    if (this.shouldFailWith) throw classifyAuthError(this.shouldFailWith)
    this.activeSessions.delete(sessionId)
  }
}

// Runtime API Factory
let activeApiClient: AuthApiClient | null = null

export function getAuthApi(): AuthApiClient {
  if (activeApiClient) return activeApiClient
  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
  if (isTauri) {
    activeApiClient = new TauriAuthApiClient()
    return activeApiClient
  }

  const isTestOrDev =
    (typeof globalThis !== 'undefined' &&
      'process' in globalThis &&
      (globalThis as { process?: { env?: { NODE_ENV?: string } } }).process?.env?.NODE_ENV === 'test') ||
    import.meta.env?.DEV ||
    import.meta.env?.MODE === 'test'

  if (isTestOrDev) {
    activeApiClient = new MockAuthApiClient()
    return activeApiClient
  }

  throw new Error('Configuration error: Authentication client is not available outside the secure Tauri runtime.')
}

export function setAuthApi(client: AuthApiClient): void {
  activeApiClient = client
}
