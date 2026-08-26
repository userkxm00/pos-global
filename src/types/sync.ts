// Synchronization & Network domain types.
// F1.17 — Offline / Online / Sync Status Indicator & SYNC_SPEC.md

export type SyncStatus = 'online' | 'offline' | 'syncing' | 'error' | 'paused'

export interface SyncStateSummary {
  status: SyncStatus
  isOnline: boolean
  isSyncing: boolean
  pendingCount: number
  failedCount: number
  lastSyncedAt: string | null
  lastError: string | null
  deviceId: string | null
}

export interface ManualSyncResult {
  success: boolean
  syncedCount: number
  remainingPending: number
  errorMessage: string | null
}
