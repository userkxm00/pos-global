// Pure domain logic and validation for Context Switching
// F1.15 — Organization / Branch / Register Context Hierarchy Invariants

import type { Organization } from '../types/organization'
import type { Branch } from '../types/branch'
import type { Register } from '../types/register'

/**
 * Validates that an Organization, Branch, and Register form a strictly consistent,
 * non-corrupted operational hierarchy.
 */
export function validateContextHierarchy(
  org: Organization | null | undefined,
  branch: Branch | null | undefined,
  register: Register | null | undefined,
): boolean {
  if (!org?.id || !branch?.id || !register?.id) {
    return false
  }

  // Branch must strictly belong to the Organization
  if (branch.organization_id !== org.id) {
    return false
  }

  // Register must strictly belong to both the Organization and the Branch
  if (register.organization_id !== org.id || register.branch_id !== branch.id) {
    return false
  }

  return true
}

/**
 * Checks whether a branch belongs to the given organization.
 */
export function isBranchCompatible(
  branch: Branch | null | undefined,
  orgId: string | null | undefined,
): boolean {
  if (!branch?.id || !orgId) return false
  return branch.organization_id === orgId
}

/**
 * Checks whether a register belongs to the given organization and branch.
 */
export function isRegisterCompatible(
  register: Register | null | undefined,
  orgId: string | null | undefined,
  branchId: string | null | undefined,
): boolean {
  if (!register?.id || !orgId || !branchId) return false
  return register.organization_id === orgId && register.branch_id === branchId
}

/**
 * Pure resolution for cascading selection when organization changes.
 * If the previously selected branch does not belong to newOrgId or is not in availableBranches,
 * it returns null (invalidating stale context).
 */
export function resolveBranchOnOrgChange(
  newOrgId: string | null | undefined,
  currentBranch: Branch | null | undefined,
  availableBranches: Branch[],
): Branch | null {
  if (!newOrgId || !currentBranch?.id) return null
  if (currentBranch.organization_id !== newOrgId) return null
  const match = availableBranches.find((b) => b.id === currentBranch.id && b.organization_id === newOrgId)
  return match || null
}

/**
 * Pure resolution for cascading selection when branch changes.
 * If the previously selected register does not belong to newBranchId or is not in availableRegisters,
 * it returns null (invalidating stale context).
 */
export function resolveRegisterOnBranchChange(
  newOrgId: string | null | undefined,
  newBranchId: string | null | undefined,
  currentRegister: Register | null | undefined,
  availableRegisters: Register[],
): Register | null {
  if (!newOrgId || !newBranchId || !currentRegister?.id) return null
  if (currentRegister.organization_id !== newOrgId || currentRegister.branch_id !== newBranchId) {
    return null
  }
  const match = availableRegisters.find(
    (r) => r.id === currentRegister.id && r.branch_id === newBranchId && r.organization_id === newOrgId,
  )
  return match || null
}

/**
 * State guard for tracking request sequence IDs to discard out-of-order responses.
 */
export interface CascadingSequenceGuard {
  orgReqSeq: number
  branchReqSeq: number
}

export function createCascadingSequenceGuard(): CascadingSequenceGuard {
  return { orgReqSeq: 0, branchReqSeq: 0 }
}

/**
 * Performs sequence-guarded organization branch fetching.
 */
export async function performGuardedOrgFetch(
  fetchBranches: (orgId: string) => Promise<Branch[]>,
  newOrgId: string,
  guard: CascadingSequenceGuard,
  onSuccess: (branches: Branch[], seq: number) => void,
): Promise<number> {
  const seq = ++guard.orgReqSeq
  ++guard.branchReqSeq // Invalidate any pending register requests
  if (!newOrgId) {
    onSuccess([], seq)
    return seq
  }
  const fetchedBranches = await fetchBranches(newOrgId)
  if (seq === guard.orgReqSeq) {
    onSuccess(fetchedBranches, seq)
  }
  return seq
}

/**
 * Performs sequence-guarded branch register fetching.
 */
export async function performGuardedBranchFetch(
  fetchRegisters: (branchId: string) => Promise<Register[]>,
  newBranchId: string,
  guard: CascadingSequenceGuard,
  onSuccess: (registers: Register[], seq: number) => void,
): Promise<number> {
  const seq = ++guard.branchReqSeq
  if (!newBranchId) {
    onSuccess([], seq)
    return seq
  }
  const fetchedRegisters = await fetchRegisters(newBranchId)
  if (seq === guard.branchReqSeq) {
    onSuccess(fetchedRegisters, seq)
  }
  return seq
}
