// Shared constants and step determination logic for Onboarding Wizard

import type { Organization } from '../../types/organization'
import type { Branch } from '../../types/branch'
import type { Register } from '../../types/register'

export type OnboardingStepId = 'organization' | 'branch' | 'register' | 'complete'

export const SUPPORTED_CURRENCIES = ['USD', 'EUR', 'SAR', 'MAD', 'EGP', 'AED', 'GBP'] as const
export type SupportedCurrency = (typeof SUPPORTED_CURRENCIES)[number]

export interface LanguageOption {
  code: string
  labelKey?: string
  fallbackLabel?: string
}

export const SUPPORTED_LANGUAGES: readonly LanguageOption[] = [
  { code: 'en', labelKey: 'languages.en' },
  { code: 'ar', labelKey: 'languages.ar' },
  { code: 'fr', labelKey: 'languages.fr' },
] as const

export function getCurrencyOptions(defaultCurrency?: string | null): string[] {
  if (!defaultCurrency) return [...SUPPORTED_CURRENCIES]
  const upper = defaultCurrency.trim().toUpperCase()
  if (!upper) return [...SUPPORTED_CURRENCIES]
  if (SUPPORTED_CURRENCIES.includes(upper as SupportedCurrency)) {
    return [...SUPPORTED_CURRENCIES]
  }
  return [upper, ...SUPPORTED_CURRENCIES]
}

export function getLanguageOptions(defaultLanguage?: string | null): LanguageOption[] {
  if (!defaultLanguage) return [...SUPPORTED_LANGUAGES]
  const trimmed = defaultLanguage.trim()
  if (!trimmed) return [...SUPPORTED_LANGUAGES]
  const exists = SUPPORTED_LANGUAGES.some(
    (lang) => lang.code.toLowerCase() === trimmed.toLowerCase(),
  )
  if (exists) {
    return [...SUPPORTED_LANGUAGES]
  }
  return [
    ...SUPPORTED_LANGUAGES,
    {
      code: trimmed,
      fallbackLabel: trimmed.toUpperCase(),
    },
  ]
}

export function determineInitialStep(
  org: Organization | null | undefined,
  branch: Branch | null | undefined,
  reg: Register | null | undefined,
): OnboardingStepId {
  if (org && branch && reg) return 'complete'
  if (org && branch) return 'register'
  if (org) return 'branch'
  return 'organization'
}
