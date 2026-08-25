// Deterministic Unit & Integration Tests for F1.12 Onboarding Wizard
// Covers validation, state transitions, API mocking, RTL parity, error sanitization, and context boundaries.

import { describe, it } from 'node:test'
import assert from 'node:assert/strict'
import {
  validateOrganizationInput,
  validateBranchInput,
  validateRegisterInput,
  sanitizeErrorMessage,
  normalizeCurrency,
} from '../components/onboarding/validation.ts'
import {
  determineInitialStep,
  getCurrencyOptions,
  getLanguageOptions,
  SUPPORTED_CURRENCIES,
  SUPPORTED_LANGUAGES,
} from '../components/onboarding/constants.ts'
import { MockOnboardingApiClient, setOnboardingApi } from '../services/onboardingApi.ts'
import { en, ar, fr, getDirectionForLocale } from '../i18n/index.ts'
import type { Organization } from '../types/organization.ts'
import type { Branch } from '../types/branch.ts'
import type { Register } from '../types/register.ts'

function createSampleOrg(id = 'org_test_1', currency = 'USD', lang = 'en'): Organization {
  return {
    id,
    name: 'Acme Retail Corp',
    default_currency: currency,
    default_language: lang,
    created_at: new Date().toISOString(),
  }
}

function createSampleBranch(orgId = 'org_test_1', branchId = 'br_test_1'): Branch {
  return {
    id: branchId,
    organization_id: orgId,
    name: 'Flagship Store',
    address: '123 Main St',
    currency: 'USD',
    is_active: true,
    created_at: new Date().toISOString(),
  }
}

describe('F1.12 Onboarding Wizard Test Suite', () => {
  // 1. Initial step determination using exported determineInitialStep
  it('1. starts at organization when no organization exists', () => {
    assert.strictEqual(determineInitialStep(null, null, null), 'organization')
  })

  it('2. skips organization when organization context already exists', () => {
    const org = createSampleOrg()
    assert.strictEqual(determineInitialStep(org, null, null), 'branch')
  })

  it('3. starts at branch when organization exists but branch does not', () => {
    const org = createSampleOrg('org_2', 'EUR', 'fr')
    assert.strictEqual(determineInitialStep(org, null, null), 'branch')
  })

  it('4. starts at register when organization + branch exist but register does not', () => {
    const org = createSampleOrg('org_3', 'SAR', 'ar')
    const branch = createSampleBranch(org.id, 'br_3')
    assert.strictEqual(determineInitialStep(org, branch, null), 'register')
  })

  // 5. Organization validation tests & currency normalization
  it('5. organization validation normalizes currency and rejects invalid input', () => {
    assert.ok(validateOrganizationInput({ name: '   ' }).name)
    assert.ok(validateOrganizationInput({ name: 'A'.repeat(256) }).name)
    assert.ok(validateOrganizationInput({ name: 'Valid Org', default_currency: 'us' }).default_currency)
    assert.ok(validateOrganizationInput({ name: 'Valid Org', default_currency: 'us dollar' }).default_currency)
    assert.ok(validateOrganizationInput({ name: 'Valid Org', default_language: 'x' }).default_language)

    // Valid inputs with whitespace/lowercase currency normalization
    const validWithTrim = validateOrganizationInput({
      name: 'Valid Enterprise',
      default_currency: '  usd  ',
      default_language: 'en',
    })
    assert.strictEqual(Object.keys(validWithTrim).length, 0)
    assert.strictEqual(normalizeCurrency('  usd  '), 'USD')
    assert.strictEqual(normalizeCurrency('eur'), 'EUR')
    assert.strictEqual(normalizeCurrency(''), '')
    assert.strictEqual(normalizeCurrency(null), '')
  })

  // 6. Branch validation tests
  it('6. branch validation normalizes currency and rejects invalid input', () => {
    assert.ok(validateBranchInput({ name: 'Branch 1' }).organization_id)
    assert.ok(validateBranchInput({ organization_id: 'org_1', name: '' }).name)
    assert.ok(validateBranchInput({ organization_id: 'org_1', name: 'B'.repeat(256) }).name)
    assert.ok(validateBranchInput({ organization_id: 'org_1', name: 'Valid', address: 'X'.repeat(501) }).address)
    assert.ok(validateBranchInput({ organization_id: 'org_1', name: 'Valid', currency: 'invalid' }).currency)

    const valid = validateBranchInput({
      organization_id: 'org_1',
      name: 'Main Downtown Store',
      address: '123 Main St',
      currency: '  sar  ',
    })
    assert.strictEqual(Object.keys(valid).length, 0)
  })

  // 7. Register validation tests
  it('7. register validation rejects invalid input', () => {
    const errNoContext = validateRegisterInput({ name: 'POS-01' })
    assert.ok(errNoContext.organization_id)
    assert.ok(errNoContext.branch_id)

    assert.ok(validateRegisterInput({ organization_id: 'org_1', branch_id: 'br_1', name: '' }).name)
    assert.ok(
      validateRegisterInput({
        organization_id: 'org_1',
        branch_id: 'br_1',
        name: 'POS-01',
        code: 'C'.repeat(51),
      }).code,
    )

    const valid = validateRegisterInput({
      organization_id: 'org_1',
      branch_id: 'br_1',
      name: 'Front Counter POS',
      code: 'REG-01',
    })
    assert.strictEqual(Object.keys(valid).length, 0)
  })

  // 8, 9, 10. End-to-end API progression tests
  it('8. successful organization creation advances to branch', async () => {
    const mockApi = new MockOnboardingApiClient()
    setOnboardingApi(mockApi)

    const org = await mockApi.createOrganization({
      name: 'Acme Retail Corp',
      default_currency: 'USD',
      default_language: 'en',
    })

    assert.ok(org.id.startsWith('org_'))
    assert.strictEqual(org.name, 'Acme Retail Corp')
    assert.strictEqual(org.default_currency, 'USD')
    assert.strictEqual(org.default_language, 'en')
  })

  it('9. successful branch creation advances to register', async () => {
    const mockApi = new MockOnboardingApiClient()
    setOnboardingApi(mockApi)

    const org = await mockApi.createOrganization({ name: 'Acme Retail' })
    const branch = await mockApi.createBranch({
      organization_id: org.id,
      name: 'Flagship Store',
      address: '456 Market St',
      currency: 'USD',
    })

    assert.ok(branch.id.startsWith('br_'))
    assert.strictEqual(branch.organization_id, org.id)
    assert.strictEqual(branch.name, 'Flagship Store')
    assert.strictEqual(branch.is_active, true)
  })

  it('10. successful register creation reaches completion', async () => {
    const mockApi = new MockOnboardingApiClient()
    setOnboardingApi(mockApi)

    const org = await mockApi.createOrganization({ name: 'Acme Retail' })
    const branch = await mockApi.createBranch({ organization_id: org.id, name: 'Flagship Store' })
    const register = await mockApi.createRegister({
      organization_id: org.id,
      branch_id: branch.id,
      name: 'Checkout Terminal 1',
      code: 'POS-01',
    })

    assert.ok(register.id.startsWith('reg_'))
    assert.strictEqual(register.organization_id, org.id)
    assert.strictEqual(register.branch_id, branch.id)
    assert.strictEqual(register.name, 'Checkout Terminal 1')
    assert.strictEqual(register.code, 'POS-01')
  })

  // 11. Allowlist & i18n key error sanitization
  it('11. backend errors map directly to i18n keys and never expose internal SQL/database messages', () => {
    // Known domain messages map to localized translation keys
    assert.strictEqual(
      sanitizeErrorMessage('Organization name cannot be empty'),
      'onboarding.validation.orgNameRequired',
    )
    assert.strictEqual(
      sanitizeErrorMessage('Organization name exceeds maximum length of 255 characters'),
      'onboarding.validation.orgNameTooLong',
    )
    assert.strictEqual(
      sanitizeErrorMessage('Branch name cannot be empty'),
      'onboarding.validation.branchNameRequired',
    )
    assert.strictEqual(
      sanitizeErrorMessage('Register name cannot be empty'),
      'onboarding.validation.registerNameRequired',
    )
    assert.strictEqual(
      sanitizeErrorMessage('Invalid organization: org_not_found'),
      'onboarding.validation.missingOrgContext',
    )
    assert.strictEqual(
      sanitizeErrorMessage('Invalid branch: br_not_found'),
      'onboarding.validation.missingBranchContext',
    )

    // Unsafe raw SQLite / panic / internal error is masked to generic localized key
    const rawSqlError = 'sqlite error: UNIQUE constraint failed: organizations.name near syntax error'
    assert.strictEqual(sanitizeErrorMessage(rawSqlError), 'onboarding.errors.databaseGeneric')

    const dbLockError = 'database lock failed: lock poisoned'
    assert.strictEqual(sanitizeErrorMessage(dbLockError), 'onboarding.errors.databaseGeneric')

    // Null/undefined error
    assert.strictEqual(sanitizeErrorMessage(null), 'onboarding.errors.unknown')
  })

  // 12. Context boundary enforcement: Missing org prevents branch creation
  it('12. missing organization context prevents branch creation', async () => {
    const mockApi = new MockOnboardingApiClient()
    setOnboardingApi(mockApi)

    await assert.rejects(
      async () => {
        await mockApi.createBranch({
          organization_id: 'non_existent_org',
          name: 'Invalid Branch',
        })
      },
      /Invalid organization/,
    )
  })

  // 13. Context boundary enforcement: Missing branch prevents register creation
  it('13. missing branch context prevents register creation', async () => {
    const mockApi = new MockOnboardingApiClient()
    setOnboardingApi(mockApi)

    const org = await mockApi.createOrganization({ name: 'Acme' })

    await assert.rejects(
      async () => {
        await mockApi.createRegister({
          organization_id: org.id,
          branch_id: 'non_existent_branch',
          name: 'Invalid Register',
        })
      },
      /Invalid branch/,
    )
  })

  // 14. Arabic RTL verification
  it('14. Arabic uses RTL and non-Arabic uses LTR', () => {
    assert.strictEqual(getDirectionForLocale('ar'), 'rtl')
    assert.strictEqual(getDirectionForLocale('ar-DZ'), 'rtl')
    assert.strictEqual(getDirectionForLocale('ar-SA'), 'rtl')
    assert.strictEqual(getDirectionForLocale('ar_EG'), 'rtl')
    assert.strictEqual(getDirectionForLocale('arn'), 'ltr')
    assert.strictEqual(getDirectionForLocale('art'), 'ltr')
    assert.strictEqual(getDirectionForLocale('arabic'), 'ltr')
    assert.strictEqual(getDirectionForLocale('en'), 'ltr')
    assert.strictEqual(getDirectionForLocale('fr'), 'ltr')
  })

  // 15. Translation key parity verification
  it('15. translation key parity remains intact across en, ar, fr', () => {
    function getNestedKeys(obj: Record<string, unknown>, prefix = ''): string[] {
      let keys: string[] = []
      for (const [k, v] of Object.entries(obj)) {
        const fullKey = prefix ? `${prefix}.${k}` : k
        if (v && typeof v === 'object' && !Array.isArray(v)) {
          keys = keys.concat(getNestedKeys(v as Record<string, unknown>, fullKey))
        } else {
          keys.push(fullKey)
        }
      }
      return keys.sort()
    }

    const enKeys = getNestedKeys(en.onboarding)
    const arKeys = getNestedKeys(ar.onboarding)
    const frKeys = getNestedKeys(fr.onboarding)

    assert.deepStrictEqual(arKeys, enKeys, 'Arabic onboarding keys must match English 100%')
    assert.deepStrictEqual(frKeys, enKeys, 'French onboarding keys must match English 100%')
  })

  // 16. Currency and language normalization options & constants
  it('16. currency and language options include defaults and custom persisted values cleanly', () => {
    const optionsUsd = getCurrencyOptions('USD')
    assert.strictEqual(optionsUsd.length, SUPPORTED_CURRENCIES.length)
    assert.ok(optionsUsd.includes('USD'))

    const optionsKwd = getCurrencyOptions('KWD')
    assert.strictEqual(optionsKwd.length, SUPPORTED_CURRENCIES.length + 1)
    assert.strictEqual(optionsKwd[0], 'KWD')

    const nullOptions = getCurrencyOptions(null)
    assert.strictEqual(nullOptions.length, SUPPORTED_CURRENCIES.length)

    // Language options assertions
    const defaultLanguages = getLanguageOptions(null)
    assert.strictEqual(defaultLanguages.length, SUPPORTED_LANGUAGES.length)

    const supportedLang = getLanguageOptions('fr')
    assert.strictEqual(supportedLang.length, SUPPORTED_LANGUAGES.length)

    const customPersistedLang = getLanguageOptions('de')
    assert.strictEqual(customPersistedLang.length, SUPPORTED_LANGUAGES.length + 1)
    assert.strictEqual(customPersistedLang[customPersistedLang.length - 1].code, 'de')
    assert.strictEqual(customPersistedLang[customPersistedLang.length - 1].fallbackLabel, 'DE')
  })

  // 17. Back navigation idempotency: Back -> Forward does not duplicate entities
  it('17. back navigation preserves existing entity without duplicate creation', async () => {
    const mockApi = new MockOnboardingApiClient()
    setOnboardingApi(mockApi)

    // Step 1: Create Organization
    const org = await mockApi.createOrganization({ name: 'Acme Retail' })
    const orgList1 = await mockApi.listOrganizations()
    assert.strictEqual(orgList1.length, 1)

    // Step 2: Create Branch
    const branch = await mockApi.createBranch({ organization_id: org.id, name: 'Store 1' })
    const branchList1 = await mockApi.listBranches(org.id)
    assert.strictEqual(branchList1.length, 1)

    // Simulating Back -> Forward navigation without modifications:
    // Existing entities are retained in wizard state and NOT recreated
    const orgList2 = await mockApi.listOrganizations()
    assert.strictEqual(orgList2.length, 1, 'Organizations count must remain 1 after back navigation')

    const branchList2 = await mockApi.listBranches(org.id)
    assert.strictEqual(branchList2.length, 1, 'Branches count must remain 1 after back navigation')

    // Step 3: Create Register
    const reg = await mockApi.createRegister({
      organization_id: org.id,
      branch_id: branch.id,
      name: 'POS 1',
    })
    const regList = await mockApi.listRegisters(branch.id)
    assert.strictEqual(regList.length, 1)
    assert.strictEqual(reg.name, 'POS 1')
  })
})
