// Authoritative Sync API Client
// F1.17 — Offline / Online / Sync Status Indicator
// Interfaces with local sync state, transactional outbox count, and cloud synchronization

import type { SyncStatus, SyncStateSummary, ManualSyncResult } from '../types/sync'

export interface SyncApiClient {
  getSyncStatus(branchId?: string): Promise<SyncStateSummary>
  triggerManualSync(branchId?: string): Promise<ManualSyncResult>
  getPendingQueueCount(branchId?: string): Promise<number>
}

export function extractInvokeErrorMessage(err: unknown): string {
  if (typeof err === 'string') return err
  if (err instanceof Error) return err.message
  return String(err)
}

// Real Tauri IPC Implementation
export class TauriSyncApiClient implements SyncApiClient {
  private async invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      return await invoke<T>(cmd, args)
    } catch (err) {
      throw new Error(extractInvokeErrorMessage(err))
    }
  }

  async getSyncStatus(branchId?: string): Promise<SyncStateSummary> {
    try {
      return await this.invoke<SyncStateSummary>('get_sync_status', { branchId })
    } catch {
      const isOnline = typeof navigator !== 'undefined' ? navigator.onLine : true
      const pendingCount = await this.getPendingQueueCount(branchId)
      const status: SyncStatus = !isOnline ? 'offline' : 'online'

      return {
        status,
        isOnline,
        isSyncing: false,
        pendingCount,
        failedCount: 0,
        lastSyncedAt: new Date().toISOString(),
        lastError: null,
        deviceId: null,
      }
    }
  }

  async triggerManualSync(branchId?: string): Promise<ManualSyncResult> {
    try {
      return await this.invoke<ManualSyncResult>('trigger_manual_sync', { branchId })
    } catch (err) {
      return {
        success: false,
        syncedCount: 0,
        remainingPending: 0,
        errorMessage: extractInvokeErrorMessage(err),
      }
    }
  }

  async getPendingQueueCount(branchId?: string): Promise<number> {
    try {
      return await this.invoke<number>('get_pending_sync_count', { branchId })
    } catch {
      return 0
    }
  }
}

// In-Memory Mock Implementation for Testing and Dev
export class MockSyncApiClient implements SyncApiClient {
  public summary: SyncStateSummary
  public shouldFailWith: string | null = null
  public delayMs: number = 0

  constructor(initialSummary?: Partial<SyncStateSummary>) {
    this.summary = {
      status: 'online',
      isOnline: true,
      isSyncing: false,
      pendingCount: 0,
      failedCount: 0,
      lastSyncedAt: new Date().toISOString(),
      lastError: null,
      deviceId: 'reg_mock_01',
      ...initialSummary,
    }
  }

  private async maybeDelay(): Promise<void> {
    if (this.delayMs > 0) {
      await new Promise((resolve) => setTimeout(resolve, this.delayMs))
    }
  }

  async getSyncStatus(_branchId?: string): Promise<SyncStateSummary> {
    await this.maybeDelay()
    if (this.shouldFailWith) throw new Error(this.shouldFailWith)
    return { ...this.summary }
  }

  async triggerManualSync(_branchId?: string): Promise<ManualSyncResult> {
    await this.maybeDelay()
    if (this.shouldFailWith) {
      this.summary.status = 'error'
      this.summary.lastError = this.shouldFailWith
      return {
        success: false,
        syncedCount: 0,
        remainingPending: this.summary.pendingCount,
        errorMessage: this.shouldFailWith,
      }
    }

    if (!this.summary.isOnline) {
      return {
        success: false,
        syncedCount: 0,
        remainingPending: this.summary.pendingCount,
        errorMessage: 'Network is offline',
      }
    }

    const synced = this.summary.pendingCount
    this.summary.pendingCount = 0
    this.summary.status = 'online'
    this.summary.isSyncing = false
    this.summary.lastSyncedAt = new Date().toISOString()
    this.summary.lastError = null

    return {
      success: true,
      syncedCount: synced,
      remainingPending: 0,
      errorMessage: null,
    }
  }

  async getPendingQueueCount(_branchId?: string): Promise<number> {
    await this.maybeDelay()
    if (this.shouldFailWith) throw new Error(this.shouldFailWith)
    return this.summary.pendingCount
  }
}

// Active singleton instance
let activeSyncClient: SyncApiClient = new TauriSyncApiClient()

export function getSyncApi(): SyncApiClient {
  return activeSyncClient
}

export function setSyncApi(client: SyncApiClient): void {
  activeSyncClient = client
}
