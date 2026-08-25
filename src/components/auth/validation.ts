// Validation and Fail-Closed Error Sanitization for Authentication & PIN/Lock
// F1.13 — Authentication screens and session lifecycle & F1.14 — Local PIN and Lock screen

import type { SignInInput, LocalSignInInput } from '../../types/auth'

export interface AuthValidationErrors {
  email?: string
  username?: string
  password?: string
  pin?: string
}

const EMAIL_REGEX = /^[^\s@]+@[^\s@]+\.[a-zA-Z0-9-]{2,}$/

export function validateOnlineInput(input: Partial<SignInInput>): AuthValidationErrors {
  const errors: AuthValidationErrors = {}
  const email = (input.email || '').trim()
  const password = input.password || ''

  if (!email) {
    errors.email = 'auth.validation.emailRequired'
  } else if (!EMAIL_REGEX.test(email)) {
    errors.email = 'auth.validation.emailInvalid'
  }

  if (!password) {
    errors.password = 'auth.validation.passwordRequired'
  }

  return errors
}

export function validateLocalInput(input: Partial<LocalSignInInput>): AuthValidationErrors {
  const errors: AuthValidationErrors = {}
  const username = (input.username || '').trim()
  const password = input.password || ''

  if (!username) {
    errors.username = 'auth.validation.usernameRequired'
  }

  if (!password) {
    errors.password = 'auth.validation.passwordRequired'
  }

  return errors
}

export function validatePinInput(pin: string): string | null {
  const trimmed = pin.trim()
  if (!trimmed) {
    return 'pin.errors.required'
  }
  if (!/^\d+$/.test(trimmed)) {
    return 'pin.errors.digitsOnly'
  }
  return null
}

interface ErrorPattern {
  token: string
  translationKey: string
}

const ERROR_PATTERNS: readonly ErrorPattern[] = [
  { token: 'invalid pin', translationKey: 'auth.errors.invalidPin' },
  { token: 'account is temporarily locked', translationKey: 'auth.errors.accountLocked' },
  { token: 'temporarily locked', translationKey: 'auth.errors.accountLocked' },
  { token: 'too many failed attempts', translationKey: 'auth.errors.accountLocked' },
  { token: 'invalid credentials', translationKey: 'auth.errors.invalidCredentials' },
  { token: 'credentials mismatch', translationKey: 'auth.errors.invalidCredentials' },
  { token: 'does not match', translationKey: 'auth.errors.invalidCredentials' },
  { token: 'user not found', translationKey: 'auth.errors.userNotFound' },
  { token: 'user account is inactive', translationKey: 'auth.errors.userInactive' },
  { token: 'branch is inactive', translationKey: 'auth.errors.branchInactive' },
  { token: 'rate limit', translationKey: 'auth.errors.rateLimited' },
  { token: 'network error', translationKey: 'auth.errors.networkError' },
  { token: 'unable to reach supabase', translationKey: 'auth.errors.networkError' },
  { token: 'service unavailable', translationKey: 'auth.errors.serviceUnavailable' },
  { token: 'session expired', translationKey: 'auth.errors.sessionExpired' },
  { token: 'session has expired', translationKey: 'auth.errors.sessionExpired' },
  { token: 'session not found', translationKey: 'auth.errors.sessionExpired' },
  { token: 'session has been revoked', translationKey: 'auth.errors.sessionRevoked' },
  { token: 'configuration error', translationKey: 'auth.errors.configurationError' },
] as const

export function sanitizeAuthErrorMessage(err: unknown): string {
  if (err === null || err === undefined) {
    return 'auth.errors.unknown'
  }

  let rawMessage = ''
  if (typeof err === 'string') {
    rawMessage = err
  } else if (err instanceof Error) {
    rawMessage = err.message
  } else if (typeof err === 'object' && 'message' in err && typeof (err as { message: unknown }).message === 'string') {
    rawMessage = (err as { message: string }).message
  }

  const normalized = rawMessage.toLowerCase()

  for (const pattern of ERROR_PATTERNS) {
    if (normalized.includes(pattern.token)) {
      return pattern.translationKey
    }
  }

  // Fail-closed default: never expose raw SQL/panic/backend internals
  return 'auth.errors.generic'
}
