// Pure helper functions for Synchronization status derivation and display
// F1.17 — Offline / Online / Sync Status Indicator & SYNC_SPEC.md

import type { SyncStatus } from '../../types/sync'

export function computeEffectiveSyncStatus(
  isOnline: boolean,
  isSyncing: boolean,
  isManualSyncing: boolean,
  syncStatus?: SyncStatus,
  lastError?: string | null,
): SyncStatus {
  if (!isOnline) return 'offline'
  if (isSyncing || isManualSyncing) return 'syncing'
  if (syncStatus === 'paused') return 'paused'
  if (syncStatus === 'error' || Boolean(lastError)) return 'error'
  return 'online'
}

export function getSyncStatusLabel(
  status: SyncStatus,
  t: (key: string) => string,
): string {
  switch (status) {
    case 'offline':
      return t('status.offline')
    case 'syncing':
      return t('status.syncing')
    case 'error':
      return t('sync.syncFailed')
    case 'paused':
      return t('sync.paused')
    case 'online':
    default:
      return t('status.online')
  }
}

export function getSyncLifecycleText(
  status: SyncStatus,
  pendingCount: number,
  t: (key: string, opts?: Record<string, unknown>) => string,
): string {
  if (status === 'syncing') return t('sync.syncing')
  if (status === 'error') return t('sync.syncFailed')
  if (status === 'paused') return t('sync.paused')
  if (pendingCount > 0) return t('status.pendingChanges', { count: pendingCount })
  return t('sync.synced')
}

export function getSyncActionButtonText(
  isBusy: boolean,
  status: SyncStatus,
  t: (key: string) => string,
): string {
  if (isBusy) return t('sync.syncingAction')
  if (status === 'error') return t('sync.retrySync')
  return t('sync.syncNow')
}
