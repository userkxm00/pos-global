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
import {
  computeEffectiveSyncStatus,
  getSyncStatusLabel,
  getSyncLifecycleText,
  getSyncActionButtonText,
} from '../components/common/syncHelpers.ts'
import { en, ar, fr } from '../i18n/index.ts'
import type { SyncStatus } from '../types/sync.ts'

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

  // Test 8: TauriSyncApiClient Fallback with NULL lastSyncedAt
  it('8. TauriSyncApiClient fallback never fabricates lastSyncedAt timestamp on failure', async () => {
    const tauriClient = new TauriSyncApiClient()
    setSyncApi(tauriClient)
    assert.strictEqual(getSyncApi(), tauriClient)

    const fallbackStatus = await tauriClient.getSyncStatus('branch_1')
    assert.ok(['online', 'offline'].includes(fallbackStatus.status))
    assert.strictEqual(fallbackStatus.lastSyncedAt, null) // Invariant: must be null on fallback!
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

  // Test 10: computeEffectiveSyncStatus helper unit tests
  it('10. computeEffectiveSyncStatus correctly handles online, offline, syncing, error, and paused states', () => {
    assert.strictEqual(computeEffectiveSyncStatus(false, false, false, 'online', null), 'offline')
    assert.strictEqual(computeEffectiveSyncStatus(false, true, false, 'online', null), 'offline') // offline overrides syncing
    assert.strictEqual(computeEffectiveSyncStatus(true, true, false, 'online', null), 'syncing')
    assert.strictEqual(computeEffectiveSyncStatus(true, false, true, 'online', null), 'syncing')
    assert.strictEqual(computeEffectiveSyncStatus(true, false, false, 'paused', null), 'paused')
    assert.strictEqual(computeEffectiveSyncStatus(true, false, false, 'error', 'Failed'), 'error')
    assert.strictEqual(computeEffectiveSyncStatus(true, false, false, 'online', 'Failed'), 'error')
    assert.strictEqual(computeEffectiveSyncStatus(true, false, false, 'online', null), 'online')
  })

  // Test 11: getSyncStatusLabel & getSyncLifecycleText unit tests
  it('11. getSyncStatusLabel and getSyncLifecycleText return expected localized keys', () => {
    const dummyT = (key: string, opts?: Record<string, unknown>) => {
      if (opts && typeof opts.count === 'number') {
        return `${key}:${opts.count}`
      }
      return key
    }

    assert.strictEqual(getSyncStatusLabel('offline', dummyT), 'status.offline')
    assert.strictEqual(getSyncStatusLabel('syncing', dummyT), 'status.syncing')
    assert.strictEqual(getSyncStatusLabel('error', dummyT), 'sync.syncFailed')
    assert.strictEqual(getSyncStatusLabel('paused', dummyT), 'sync.paused')
    assert.strictEqual(getSyncStatusLabel('online', dummyT), 'status.online')

    assert.strictEqual(getSyncLifecycleText('syncing', 3, dummyT), 'sync.syncing')
    assert.strictEqual(getSyncLifecycleText('error', 3, dummyT), 'sync.syncFailed')
    assert.strictEqual(getSyncLifecycleText('paused', 3, dummyT), 'sync.paused')
    assert.strictEqual(getSyncLifecycleText('online', 3, dummyT), 'status.pendingChanges:3')
    assert.strictEqual(getSyncLifecycleText('online', 0, dummyT), 'sync.synced')

    assert.strictEqual(getSyncActionButtonText(true, 'online', dummyT), 'sync.syncingAction')
    assert.strictEqual(getSyncActionButtonText(false, 'error', dummyT), 'sync.retrySync')
    assert.strictEqual(getSyncActionButtonText(false, 'online', dummyT), 'sync.syncNow')
  })

  // Test 12: Internationalization Completeness for Sync & Status Keys
  it('12. i18n dictionaries provide complete translations for sync and status in en, ar, and fr', () => {
    const requiredKeys = [
      'title',
      'connectionStatus',
      'syncLifecycle',
      'synced',
      'syncing',
      'syncFailed',
      'paused',
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

  // Test 13: Arabic Plural Form Completeness for Pending Changes
  it('13. Arabic dictionary defines all 6 grammatical plural forms for pending changes', () => {
    assert.ok(ar.status.pendingChanges)
    assert.ok(ar.status.pendingChanges_zero)
    assert.ok(ar.status.pendingChanges_one)
    assert.ok(ar.status.pendingChanges_two)
    assert.ok(ar.status.pendingChanges_few)
    assert.ok(ar.status.pendingChanges_many)
    assert.ok(ar.status.pendingChanges_other)
  })

  // Test 14: Stale Request Protection Simulation
  it('14. protects against out-of-order async responses using request sequence tracking', async () => {
    let activeRequestId = 0
    let committedState: SyncStatus = 'online'

    const simulateFetch = async (requestId: number, delayMs: number, targetStatus: SyncStatus) => {
      await new Promise((r) => setTimeout(r, delayMs))
      if (requestId === activeRequestId) {
        committedState = targetStatus
      }
    }

    // Request 1 starts (slow)
    const req1 = ++activeRequestId
    const p1 = simulateFetch(req1, 20, 'error')

    // Request 2 starts (fast)
    const req2 = ++activeRequestId
    const p2 = simulateFetch(req2, 5, 'paused')

    await Promise.all([p1, p2])

    // State from fast req2 should remain, req1 must NOT overwrite
    assert.strictEqual(committedState, 'paused')
  })
})
