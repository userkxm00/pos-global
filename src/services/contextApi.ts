// Authoritative Tauri IPC Client for Organization / Branch / Register Context Operations
// F1.15 — Context Switcher API Boundary
// Invokes production Tauri commands in src-tauri/src/commands/{organization,branch,register}.rs

import type { Organization } from '../types/organization'
import type { Branch } from '../types/branch'
import type { Register } from '../types/register'

export interface ContextApiClient {
  listOrganizations(): Promise<Organization[]>
  listBranches(organizationId: string): Promise<Branch[]>
  listRegisters(branchId: string): Promise<Register[]>
}

export function extractInvokeErrorMessage(err: unknown): string {
  if (typeof err === 'string') return err
  if (err instanceof Error) return err.message
  return String(err)
}

// Real Tauri IPC Implementation
export class TauriContextApiClient implements ContextApiClient {
  private async invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      return await invoke<T>(cmd, args)
    } catch (err) {
      throw new Error(extractInvokeErrorMessage(err))
    }
  }

  async listOrganizations(): Promise<Organization[]> {
    return this.invoke<Organization[]>('list_organizations')
  }

  async listBranches(organizationId: string): Promise<Branch[]> {
    return this.invoke<Branch[]>('list_branches', { organizationId })
  }

  async listRegisters(branchId: string): Promise<Register[]> {
    return this.invoke<Register[]>('list_registers', { branchId })
  }
}

// In-Memory Mock Implementation for Tests and Isolation
export class MockContextApiClient implements ContextApiClient {
  public organizations: Organization[] = []
  public branches: Branch[] = []
  public registers: Register[] = []
  public shouldFailWith: string | null = null
  public delayMs: number = 0

  private async maybeDelay(): Promise<void> {
    if (this.delayMs > 0) {
      await new Promise((resolve) => setTimeout(resolve, this.delayMs))
    }
  }

  async listOrganizations(): Promise<Organization[]> {
    await this.maybeDelay()
    if (this.shouldFailWith) throw new Error(this.shouldFailWith)
    return [...this.organizations]
  }

  async listBranches(organizationId: string): Promise<Branch[]> {
    await this.maybeDelay()
    if (this.shouldFailWith) throw new Error(this.shouldFailWith)
    return this.branches.filter((b) => b.organization_id === organizationId)
  }

  async listRegisters(branchId: string): Promise<Register[]> {
    await this.maybeDelay()
    if (this.shouldFailWith) throw new Error(this.shouldFailWith)
    return this.registers.filter((r) => r.branch_id === branchId)
  }
}

// Active singleton instance
let activeClient: ContextApiClient = new TauriContextApiClient()

export function getContextApi(): ContextApiClient {
  return activeClient
}

export function setContextApi(client: ContextApiClient): void {
  activeClient = client
}
