// Deterministic Unit & Integration Tests for F1.17: Offline / Online / Sync Status Indicator
// Tests sync status states, transactional queue counts, manual sync trigger, fallback behavior, accessibility, and i18n completeness.

import { describe, it, beforeEach } from 'node:test'
import assert from 'node:assert/strict'
import {
  MockSyncApiClient,
  TauriSyncApiClient,
  extractInvokeErrorMessage,
  getSyncApi,
  setSyncApi,
} from '../services/syncApi.ts'
import { en, ar, fr } from '../i18n/index.ts'
import type { SyncStateSummary, SyncStatus } from '../types/sync.ts'

describe('F1.17 Sync Status Indicator Test Suite', () => {
  let mockApi: MockSyncApiClient

  beforeEach(() => {
    mockApi = new MockSyncApiClient({
      status: 'online',
      isOnline: true,
      isSyncing: false,
      pendingCount: 0,
      failedCount: 0,
      lastSyncedAt: '2026-08-26T00:00:00.000Z',
      lastError: null,
      deviceId: 'reg_terminal_01',
    })
    setSyncApi(mockApi)
  })

  // Test 1: Authoritative Online & Synchronized State
  it('1. returns online and synchronized status when connected with 0 pending changes', async () => {
    const status = await mockApi.getSyncStatus('branch_1')
    assert.strictEqual(status.status, 'online')
    assert.strictEqual(status.isOnline, true)
    assert.strictEqual(status.isSyncing, false)
    assert.strictEqual(status.pendingCount, 0)
    assert.strictEqual(status.failedCount, 0)
    assert.strictEqual(status.lastError, null)
    assert.strictEqual(status.deviceId, 'reg_terminal_01')
  })

  // Test 2: Offline State Determination
  it('2. returns offline status when network connectivity is lost', async () => {
    mockApi.summary.isOnline = false
    mockApi.summary.status = 'offline'

    const status = await mockApi.getSyncStatus('branch_1')
    assert.strictEqual(status.status, 'offline')
    assert.strictEqual(status.isOnline, false)
  })

  // Test 3: Pending Outbox Changes Tracking
  it('3. accurately tracks pending outbox changes count', async () => {
    mockApi.summary.pendingCount = 7

    const count = await mockApi.getPendingQueueCount('branch_1')
    assert.strictEqual(count, 7)

    const status = await mockApi.getSyncStatus('branch_1')
    assert.strictEqual(status.pendingCount, 7)
  })

  // Test 4: Manual Sync Execution Success
  it('4. triggerManualSync flushes pending changes and updates sync timestamp on success', async () => {
    mockApi.summary.pendingCount = 5
    mockApi.summary.isOnline = true

    const result = await mockApi.triggerManualSync('branch_1')
    assert.strictEqual(result.success, true)
    assert.strictEqual(result.syncedCount, 5)
    assert.strictEqual(result.remainingPending, 0)
    assert.strictEqual(result.errorMessage, null)

    const afterStatus = await mockApi.getSyncStatus('branch_1')
    assert.strictEqual(afterStatus.pendingCount, 0)
    assert.strictEqual(afterStatus.status, 'online')
    assert.ok(afterStatus.lastSyncedAt !== null)
  })

  // Test 5: Manual Sync Execution Blocked When Offline
  it('5. triggerManualSync safely fails when network is offline without discarding local outbox items', async () => {
    mockApi.summary.isOnline = false
    mockApi.summary.status = 'offline'
    mockApi.summary.pendingCount = 3

    const result = await mockApi.triggerManualSync('branch_1')
    assert.strictEqual(result.success, false)
    assert.strictEqual(result.syncedCount, 0)
    assert.strictEqual(result.remainingPending, 3)
    assert.strictEqual(result.errorMessage, 'Network is offline')

    const afterStatus = await mockApi.getSyncStatus('branch_1')
    assert.strictEqual(afterStatus.pendingCount, 3)
  })

  // Test 6: Sync Failure Simulation & Error State
  it('6. triggerManualSync captures failure diagnostics and transitions to error state', async () => {
    mockApi.shouldFailWith = 'Cloud gateway timeout: HTTP 504'
    mockApi.summary.pendingCount = 4

    const result = await mockApi.triggerManualSync('branch_1')
    assert.strictEqual(result.success, false)
    assert.strictEqual(result.syncedCount, 0)
    assert.strictEqual(result.remainingPending, 4)
    assert.strictEqual(result.errorMessage, 'Cloud gateway timeout: HTTP 504')

    mockApi.shouldFailWith = null
    const afterStatus = await mockApi.getSyncStatus('branch_1')
    assert.strictEqual(afterStatus.status, 'error')
    assert.strictEqual(afterStatus.lastError, 'Cloud gateway timeout: HTTP 504')
  })

  // Test 7: Mock Delay and Error Propagation
  it('7. MockSyncApiClient propagates simulated errors and delays deterministically', async () => {
    mockApi.delayMs = 2
    mockApi.shouldFailWith = 'Database sync table locked'

    await assert.rejects(() => mockApi.getSyncStatus('branch_1'), /Database sync table locked/)
    await assert.rejects(() => mockApi.getPendingQueueCount('branch_1'), /Database sync table locked/)
  })

  // Test 8: TauriSyncApiClient Fallback
  it('8. TauriSyncApiClient handles invocation environment gracefully outside Tauri', async () => {
    const tauriClient = new TauriSyncApiClient()
    setSyncApi(tauriClient)
    assert.strictEqual(getSyncApi(), tauriClient)

    const fallbackStatus = await tauriClient.getSyncStatus('branch_1')
    assert.ok(['online', 'offline'].includes(fallbackStatus.status))
    assert.strictEqual(fallbackStatus.failedCount, 0)

    const fallbackCount = await tauriClient.getPendingQueueCount('branch_1')
    assert.strictEqual(fallbackCount, 0)

    const manualResult = await tauriClient.triggerManualSync('branch_1')
    assert.strictEqual(manualResult.success, false)

    // Reset back to mockApi
    setSyncApi(mockApi)
  })

  // Test 9: Error Message Extraction
  it('9. extractInvokeErrorMessage extracts safe messages from various error shapes', () => {
    assert.strictEqual(extractInvokeErrorMessage('Plain string error'), 'Plain string error')
    assert.strictEqual(extractInvokeErrorMessage(new Error('Typed Error message')), 'Typed Error message')
    assert.strictEqual(extractInvokeErrorMessage({ custom: 123 }), '[object Object]')
  })

  // Test 10: Status Derivation Logic
  it('10. derived effective sync status correctly prioritizes offline > syncing > error > online', () => {
    function deriveStatus(
      isOnline: boolean,
      isSyncing: boolean,
      syncStatus?: SyncStatus,
      lastError?: string | null,
    ): SyncStatus {
      if (!isOnline) return 'offline'
      if (isSyncing) return 'syncing'
      if (syncStatus === 'error' || lastError) return 'error'
      return 'online'
    }

    assert.strictEqual(deriveStatus(false, false, 'online', null), 'offline')
    assert.strictEqual(deriveStatus(false, true, 'syncing', null), 'offline') // offline overrides syncing
    assert.strictEqual(deriveStatus(true, true, 'online', null), 'syncing')
    assert.strictEqual(deriveStatus(true, false, 'error', 'Error text'), 'error')
    assert.strictEqual(deriveStatus(true, false, 'online', null), 'online')
  })

  // Test 11: Internationalization Completeness for Sync & Status Keys
  it('11. i18n dictionaries provide complete translations for sync and status in en, ar, and fr', () => {
    const requiredKeys = [
      'title',
      'connectionStatus',
      'syncLifecycle',
      'synced',
      'syncing',
      'syncFailed',
      'pendingChanges',
      'lastSynced',
      'neverSynced',
      'terminalId',
      'syncNow',
      'syncingAction',
      'retrySync',
      'closeModal',
      'syncSuccess',
      'syncError',
      'offlineNotice',
      'pendingCountLabel',
    ] as const

    for (const key of requiredKeys) {
      assert.ok(en.sync[key], `en missing sync key: ${key}`)
      assert.ok(ar.sync[key], `ar missing sync key: ${key}`)
      assert.ok(fr.sync[key], `fr missing sync key: ${key}`)
    }

    // Status basic keys
    assert.ok(en.status.online)
    assert.ok(ar.status.online)
    assert.ok(fr.status.online)

    assert.ok(en.status.offline)
    assert.ok(ar.status.offline)
    assert.ok(fr.status.offline)

    assert.ok(en.status.syncing)
    assert.ok(ar.status.syncing)
    assert.ok(fr.status.syncing)
  })

  // Test 12: Arabic Plural Form Completeness for Pending Changes
  it('12. Arabic dictionary defines all 6 grammatical plural forms for pending changes', () => {
    assert.ok(ar.status.pendingChanges_zero)
    assert.ok(ar.status.pendingChanges_one)
    assert.ok(ar.status.pendingChanges_two)
    assert.ok(ar.status.pendingChanges_few)
    assert.ok(ar.status.pendingChanges_many)
    assert.ok(ar.status.pendingChanges_other)
  })
})
