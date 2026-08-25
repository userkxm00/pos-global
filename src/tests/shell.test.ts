import test from 'node:test'
import assert from 'node:assert/strict'
import { en, ar, fr, getDirectionForLocale, supportedLocales } from '../i18n/index.ts'

test('i18n direction calculation conforms to language rules', () => {
  assert.equal(getDirectionForLocale('en'), 'ltr')
  assert.equal(getDirectionForLocale('fr'), 'ltr')
  assert.equal(getDirectionForLocale('ar'), 'rtl')
  assert.equal(getDirectionForLocale('ar-DZ'), 'rtl')
  assert.equal(getDirectionForLocale('ar-SA'), 'rtl')
  assert.equal(getDirectionForLocale('arn'), 'ltr')
  assert.equal(getDirectionForLocale('art'), 'ltr')
  assert.equal(getDirectionForLocale('arabic'), 'ltr')
  assert.equal(getDirectionForLocale('unknown'), 'ltr')
})

test('supported locales contains en, ar, fr', () => {
  assert.deepEqual([...supportedLocales], ['en', 'ar', 'fr'])
})

test('all required translation keys exist across all supported locales', () => {
  const locales = [
    { code: 'en', dict: en },
    { code: 'ar', dict: ar },
    { code: 'fr', dict: fr },
  ]

  for (const { code, dict } of locales) {
    // App & accessibility
    assert.ok(dict.app.name, `Missing app.name in ${code}`)
    assert.ok(dict.app.skipToContent, `Missing app.skipToContent in ${code}`)
    assert.ok(dict.app.breadcrumb, `Missing app.breadcrumb in ${code}`)

    // Navigation sections & items
    assert.ok(dict.nav.sections.operations, `Missing operations in ${code}`)
    assert.ok(dict.nav.sections.management, `Missing management in ${code}`)
    assert.ok(dict.nav.sections.administration, `Missing administration in ${code}`)
    assert.ok(dict.nav.items.pos, `Missing pos item in ${code}`)
    assert.ok(dict.nav.items.shifts, `Missing shifts item in ${code}`)
    assert.ok(dict.nav.items.inventory, `Missing inventory item in ${code}`)
    assert.ok(dict.nav.items.customers, `Missing customers item in ${code}`)
    assert.ok(dict.nav.items.reports, `Missing reports item in ${code}`)
    assert.ok(dict.nav.items.users, `Missing users item in ${code}`)
    assert.ok(dict.nav.items.tenants, `Missing tenants item in ${code}`)
    assert.ok(dict.nav.items.settings, `Missing settings item in ${code}`)

    // State views
    assert.ok(dict.states.loading.title, `Missing loading.title in ${code}`)
    assert.ok(dict.states.empty.title, `Missing empty.title in ${code}`)
    assert.ok(dict.states.error.title, `Missing error.title in ${code}`)
    assert.ok(dict.states.permissionDenied.title, `Missing permissionDenied.title in ${code}`)

    // Status & Offline
    assert.ok(dict.status.online, `Missing status.online in ${code}`)
    assert.ok(dict.status.offline, `Missing status.offline in ${code}`)
    assert.ok(dict.status.syncing, `Missing status.syncing in ${code}`)
    assert.ok(dict.status.offlineBanner, `Missing offlineBanner in ${code}`)

    // Languages selection
    assert.ok(dict.languages.select, `Missing languages.select in ${code}`)
  }
})

test('Arabic plural forms exist for pendingChanges', () => {
  assert.ok(ar.status.pendingChanges_zero)
  assert.ok(ar.status.pendingChanges_one)
  assert.ok(ar.status.pendingChanges_two)
  assert.ok(ar.status.pendingChanges_few)
  assert.ok(ar.status.pendingChanges_many)
  assert.ok(ar.status.pendingChanges_other)
})

test('keyboard shortcuts map to appropriate POS actions', () => {
  assert.equal(en.shortcuts.pos, 'F1')
  assert.equal(en.shortcuts.shifts, 'F2')
  assert.equal(en.shortcuts.lock, 'F9')
})

test('permission denied message contains parameter placeholder for dynamic permission injection', () => {
  assert.match(en.states.permissionDenied.description, /\{\{permission\}\}/)
  assert.match(ar.states.permissionDenied.description, /\{\{permission\}\}/)
  assert.match(fr.states.permissionDenied.description, /\{\{permission\}\}/)
})

test('error and state descriptions do not contain hardcoded credentials or secret patterns', () => {
  const secretPatterns = [/password/i, /secret/i, /bearer/i, /api[_-]?key/i, /token/i]
  const errorEn = en.states.error.description

  for (const pattern of secretPatterns) {
    assert.equal(
      pattern.test(errorEn),
      false,
      `Error description must not contain credential pattern ${pattern.source}`,
    )
  }
})
