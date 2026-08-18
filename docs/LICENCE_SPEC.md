# License Specification

## Security model
License state is signed by a dedicated license-signing key. The desktop application contains only the public verification key. License and updater keys are always separate.

## Entitlements
A license identifies plan, organization, allowed devices/branches, issue time, expiry/grace policy and feature entitlements. The desktop verifies signature and policy locally.

## Lifecycle
Purchase → issue → activate device → verify entitlement → refresh → offline grace → revoke/renew/reset device.

## Offline
A previously valid entitlement may continue during the configured offline grace period. Clock manipulation and replay must be detected or bounded. A normal POS sale never requires a network call.

## Server boundary
License issuance, revocation, activation limits and billing state are server-side operations. Private signing material never ships to the desktop or repository.