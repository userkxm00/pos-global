// Authoritative Tauri IPC Client for Onboarding Domain Operations
// Invokes production Tauri commands in src-tauri/src/commands/{organization,branch,register}.rs

import type { CreateOrganizationInput, Organization } from '../types/organization'
import type { CreateBranchInput, Branch } from '../types/branch'
import type { CreateRegisterInput, Register } from '../types/register'

export interface OnboardingApiClient {
  createOrganization(input: CreateOrganizationInput): Promise<Organization>
  listOrganizations(): Promise<Organization[]>
  createBranch(input: CreateBranchInput): Promise<Branch>
  listBranches(organizationId: string): Promise<Branch[]>
  createRegister(input: CreateRegisterInput): Promise<Register>
  listRegisters(branchId: string): Promise<Register[]>
}

function extractInvokeErrorMessage(err: unknown): string {
  if (typeof err === 'string') return err
  if (err instanceof Error) return err.message
  return String(err)
}

// Real Tauri IPC Implementation
class TauriOnboardingApiClient implements OnboardingApiClient {
  private async invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      return await invoke<T>(cmd, args)
    } catch (err) {
      throw new Error(extractInvokeErrorMessage(err))
    }
  }

  async createOrganization(input: CreateOrganizationInput): Promise<Organization> {
    return this.invoke<Organization>('create_organization', { request: input })
  }

  async listOrganizations(): Promise<Organization[]> {
    return this.invoke<Organization[]>('list_organizations')
  }

  async createBranch(input: CreateBranchInput): Promise<Branch> {
    return this.invoke<Branch>('create_branch', { request: input })
  }

  async listBranches(organizationId: string): Promise<Branch[]> {
    return this.invoke<Branch[]>('list_branches', { organizationId })
  }

  async createRegister(input: CreateRegisterInput): Promise<Register> {
    return this.invoke<Register>('create_register', { request: input })
  }

  async listRegisters(branchId: string): Promise<Register[]> {
    return this.invoke<Register[]>('list_registers', { branchId })
  }
}

// In-Memory Mock Implementation for Tests and Browser Previews
export class MockOnboardingApiClient implements OnboardingApiClient {
  private readonly organizations: Organization[] = []
  private readonly branches: Branch[] = []
  private readonly registers: Register[] = []
  public shouldFailWith: string | null = null

  async createOrganization(input: CreateOrganizationInput): Promise<Organization> {
    if (this.shouldFailWith) throw new Error(this.shouldFailWith)
    const org: Organization = {
      id: `org_${crypto.randomUUID()}`,
      name: input.name.trim(),
      default_currency: input.default_currency || 'USD',
      default_language: input.default_language || 'en',
      created_at: new Date().toISOString(),
    }
    this.organizations.push(org)
    return org
  }

  async listOrganizations(): Promise<Organization[]> {
    if (this.shouldFailWith) throw new Error(this.shouldFailWith)
    return [...this.organizations]
  }

  async createBranch(input: CreateBranchInput): Promise<Branch> {
    if (this.shouldFailWith) throw new Error(this.shouldFailWith)
    const org = this.organizations.find((o) => o.id === input.organization_id)
    if (!org) throw new Error(`Invalid organization: Organization '${input.organization_id}' not found`)

    const branch: Branch = {
      id: `br_${crypto.randomUUID()}`,
      organization_id: input.organization_id,
      name: input.name.trim(),
      address: input.address || null,
      currency: input.currency || org.default_currency,
      is_active: input.is_active ?? true,
      created_at: new Date().toISOString(),
    }
    this.branches.push(branch)
    return branch
  }

  async listBranches(organizationId: string): Promise<Branch[]> {
    if (this.shouldFailWith) throw new Error(this.shouldFailWith)
    return this.branches.filter((b) => b.organization_id === organizationId)
  }

  async createRegister(input: CreateRegisterInput): Promise<Register> {
    if (this.shouldFailWith) throw new Error(this.shouldFailWith)
    const branch = this.branches.find((b) => b.id === input.branch_id)
    if (!branch) throw new Error(`Invalid branch: Branch '${input.branch_id}' not found`)
    if (branch.organization_id !== input.organization_id) {
      throw new Error(`Invalid branch: Branch '${input.branch_id}' does not belong to organization '${input.organization_id}'`)
    }

    const reg: Register = {
      id: `reg_${crypto.randomUUID()}`,
      organization_id: input.organization_id,
      branch_id: input.branch_id,
      name: input.name.trim(),
      code: input.code || null,
      is_active: input.is_active ?? true,
      created_at: new Date().toISOString(),
    }
    this.registers.push(reg)
    return reg
  }

  async listRegisters(branchId: string): Promise<Register[]> {
    if (this.shouldFailWith) throw new Error(this.shouldFailWith)
    return this.registers.filter((r) => r.branch_id === branchId)
  }
}

// Active singleton instance
let activeClient: OnboardingApiClient = new TauriOnboardingApiClient()

export function getOnboardingApi(): OnboardingApiClient {
  return activeClient
}

export function setOnboardingApi(client: OnboardingApiClient): void {
  activeClient = client
}
