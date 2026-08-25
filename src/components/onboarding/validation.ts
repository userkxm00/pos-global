// Client-side validation and sanitization logic for Onboarding Wizard.
// Mirrors domain rules in src-tauri/src/{organization,branch,register}/mod.rs

import type { CreateOrganizationInput } from '../../types/organization'
import type { CreateBranchInput } from '../../types/branch'
import type { CreateRegisterInput } from '../../types/register'

export interface ValidationErrors {
  [field: string]: string
}

const CURRENCY_REGEX = /^[A-Z]{3}$/
const LANGUAGE_REGEX = /^[a-zA-Z0-9_-]{2,10}$/

export function normalizeCurrency(currency?: string | null): string {
  return currency ? currency.trim().toUpperCase() : ''
}

export function validateOrganizationInput(input: Partial<CreateOrganizationInput>): ValidationErrors {
  const errors: ValidationErrors = {}

  const trimmedName = input.name?.trim() || ''
  if (!trimmedName) {
    errors.name = 'onboarding.validation.orgNameRequired'
  } else if (trimmedName.length > 255) {
    errors.name = 'onboarding.validation.orgNameTooLong'
  }

  if (input.default_currency !== undefined && input.default_currency !== null) {
    const normalizedCurrency = normalizeCurrency(input.default_currency)
    if (!CURRENCY_REGEX.test(normalizedCurrency)) {
      errors.default_currency = 'onboarding.validation.currencyInvalid'
    }
  }

  if (input.default_language) {
    const trimmedLang = input.default_language.trim()
    if (!LANGUAGE_REGEX.test(trimmedLang)) {
      errors.default_language = 'onboarding.validation.languageInvalid'
    }
  }

  return errors
}

export function validateBranchInput(input: Partial<CreateBranchInput>): ValidationErrors {
  const errors: ValidationErrors = {}

  if (!input.organization_id?.trim()) {
    errors.organization_id = 'onboarding.validation.missingOrgContext'
  }

  const trimmedName = input.name?.trim() || ''
  if (!trimmedName) {
    errors.name = 'onboarding.validation.branchNameRequired'
  } else if (trimmedName.length > 255) {
    errors.name = 'onboarding.validation.branchNameTooLong'
  }

  if (input.address && input.address.trim().length > 500) {
    errors.address = 'onboarding.validation.addressTooLong'
  }

  if (input.currency !== undefined && input.currency !== null) {
    const normalizedCurrency = normalizeCurrency(input.currency)
    if (!CURRENCY_REGEX.test(normalizedCurrency)) {
      errors.currency = 'onboarding.validation.currencyInvalid'
    }
  }

  return errors
}

export function validateRegisterInput(input: Partial<CreateRegisterInput>): ValidationErrors {
  const errors: ValidationErrors = {}

  if (!input.organization_id?.trim()) {
    errors.organization_id = 'onboarding.validation.missingOrgContext'
  }

  if (!input.branch_id?.trim()) {
    errors.branch_id = 'onboarding.validation.missingBranchContext'
  }

  const trimmedName = input.name?.trim() || ''
  if (!trimmedName) {
    errors.name = 'onboarding.validation.registerNameRequired'
  } else if (trimmedName.length > 255) {
    errors.name = 'onboarding.validation.registerNameTooLong'
  }

  if (input.code && input.code.trim().length > 50) {
    errors.code = 'onboarding.validation.registerCodeTooLong'
  }

  return errors
}

// Explicit mapping from backend domain error strings to safe localized i18n translation keys
const BACKEND_ERROR_TO_I18N_KEY: Readonly<Record<string, string>> = {
  'Organization name cannot be empty': 'onboarding.validation.orgNameRequired',
  'Organization name exceeds maximum length of 255 characters': 'onboarding.validation.orgNameTooLong',
  'Branch name cannot be empty': 'onboarding.validation.branchNameRequired',
  'Branch name exceeds maximum length of 255 characters': 'onboarding.validation.branchNameTooLong',
  'Register name cannot be empty': 'onboarding.validation.registerNameRequired',
  'Register name exceeds maximum length of 255 characters': 'onboarding.validation.registerNameTooLong',
  'Invalid organization': 'onboarding.validation.missingOrgContext',
  'Invalid branch': 'onboarding.validation.missingBranchContext',
}

const KNOWN_I18N_KEYS: ReadonlySet<string> = new Set([
  'onboarding.validation.orgNameRequired',
  'onboarding.validation.orgNameTooLong',
  'onboarding.validation.currencyInvalid',
  'onboarding.validation.languageInvalid',
  'onboarding.validation.missingOrgContext',
  'onboarding.validation.branchNameRequired',
  'onboarding.validation.branchNameTooLong',
  'onboarding.validation.addressTooLong',
  'onboarding.validation.missingBranchContext',
  'onboarding.validation.registerNameRequired',
  'onboarding.validation.registerNameTooLong',
  'onboarding.validation.registerCodeTooLong',
  'onboarding.errors.databaseGeneric',
  'onboarding.errors.unknown',
  'onboarding.errors.orgAlreadyExists',
  'onboarding.errors.orgNotFound',
  'onboarding.errors.branchNotFound',
])

function extractRawErrorMessage(error: unknown): string {
  if (!error) return ''
  if (typeof error === 'string') return error.trim()
  if (error instanceof Error) return error.message.trim()
  if (
    typeof error === 'object' &&
    'message' in error &&
    typeof (error as { message?: unknown }).message === 'string'
  ) {
    return ((error as { message: string }).message).trim()
  }
  return ''
}

export function sanitizeErrorMessage(error: unknown): string {
  if (!error) return 'onboarding.errors.unknown'
  const message = extractRawErrorMessage(error)
  if (!message) return 'onboarding.errors.unknown'

  // If already an authoritative i18n key, pass through
  if (KNOWN_I18N_KEYS.has(message)) {
    return message
  }

  // Exact match against known backend error messages
  if (message in BACKEND_ERROR_TO_I18N_KEY) {
    return BACKEND_ERROR_TO_I18N_KEY[message]
  }

  // Parameterized prefix match for domain errors
  if (message.startsWith('Invalid organization')) {
    return 'onboarding.validation.missingOrgContext'
  }
  if (message.startsWith('Invalid branch')) {
    return 'onboarding.validation.missingBranchContext'
  }

  // Fail-closed default: map all unknown/internal/SQL/panic errors to generic safe localized key
  return 'onboarding.errors.databaseGeneric'
}
