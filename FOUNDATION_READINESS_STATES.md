# Foundation Readiness States

This document defines the only valid readiness states for the repository. A higher state must never be inferred from a lower state.

## 1. FOUNDATION_DESIGNED

The architecture, domain contracts, execution plan, backlog, agent operating system, security model, sync contract, product/UI contracts, release contract, and required foundation scaffolding exist and are internally reviewable.

**Meaning:** designed, not yet proven by CI/evidence.

## 2. FOUNDATION_VERIFIED

`FOUNDATION_DESIGNED` plus all Foundation Gate evidence is green for the exact head commit:

- frontend build passes
- Rust check passes
- Rust tests pass
- migration verification passes
- dependency/security review is recorded
- secret scanning is clean or every finding is explicitly dispositioned
- repository/spec consistency check passes

**Meaning:** the foundation is technically verified.

## 3. AGENT_IMPLEMENTATION_READY

`FOUNDATION_VERIFIED` plus:

- Definition of Ready is satisfied
- Phase 1 backlog is unambiguous
- agent state is initialized
- no unresolved decision can change the Phase 1 architecture
- implementation/review/evidence prompts are available

**Meaning:** an autonomous coding agent may begin implementation one approved task at a time.

## 4. PRODUCTION_READY

All required product capabilities are implemented and verified, including security, data integrity, sync, licensing, update, backup/recovery, hardware, E2E, performance, and applicable jurisdiction requirements.

**Meaning:** the software can enter controlled production release.

## 5. LAUNCH_READY

`PRODUCTION_READY` plus commercial, operational, support, release-signing, website, billing, legal, documentation, monitoring, and rollout gates are complete for the selected launch markets.

**Meaning:** public launch is approved.

## Non-negotiable rule

`mergeable=true` on GitHub is not evidence of `FOUNDATION_VERIFIED`. GitHub mergeability only describes whether the branch can be merged mechanically. The repository's Foundation Gate is authoritative for readiness.
