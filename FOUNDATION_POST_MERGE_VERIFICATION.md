# Post-Merge Foundation Verification

This document defines the verification procedure that runs after the Foundation PR has been merged into `foundation/v2`.

## Purpose

A green pull-request check proves the tested PR head. A subsequent merge creates a new commit, so `foundation/v2` must receive its own CI run before the repository can be treated as `FOUNDATION_VERIFIED`.

## Authoritative evidence

The authoritative evidence for the exact commit is the GitHub Actions `foundation-gate-evidence` job and its uploaded artifact. The artifact records:

- repository
- exact commit SHA
- workflow run URL and run ID
- event and branch
- required job results
- timestamp

Do not copy evidence from an older PR head to the merged commit.

## Verification sequence

1. Merge approved Foundation changes into `foundation/v2`.
2. Wait for the push-triggered CI run on the resulting `foundation/v2` commit.
3. Confirm frontend, Rust, Rust tests/migrations, secret scan and foundation specification validation are green.
4. Confirm the `foundation-gate-evidence` job is green and its artifact names the same `GITHUB_SHA` as the `foundation/v2` head.
5. Review dependency findings. A high/critical finding must have an explicit disposition; it may not be hidden by `continue-on-error`.
6. Only then transition the repository from `FOUNDATION_DESIGNED` to `FOUNDATION_VERIFIED`.
7. `AGENT_IMPLEMENTATION_READY` additionally requires the Phase 1 backlog, agent state, prompts and unresolved-decision gates to be ready.

## Important distinction

`mergeable=true`, a successful PR run, or a successful merge action is not itself a Foundation Gate pass. The exact post-merge head must have current evidence.
