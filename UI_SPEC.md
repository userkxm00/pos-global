# UI / UX SPECIFICATION

## Principles

- Fast at the point of sale.
- Clear hierarchy and low cognitive load.
- Keyboard and barcode friendly.
- Accessible by default.
- Consistent across modules.
- Offline state is visible but non-blocking.
- Destructive/financial actions require appropriate confirmation and authorization.

## Design system

Create a tokenized design system for typography, spacing, radii, elevation, states, focus, motion, density and responsive breakpoints. Components must use tokens rather than scattered magic values.

## Core components

Button, icon button, input, select, combobox, date/time picker, currency input, quantity input, barcode input, table, virtualized table where needed, modal, drawer, toast, alert, confirmation dialog, tabs, command palette, pagination, empty state, loading skeleton, error state, offline indicator, sync indicator, permission gate, product card, variant matrix, payment keypad, receipt preview.

## POS UX

- Barcode input can receive scanner bursts without requiring mouse interaction.
- Search is fast and forgiving.
- Quantity and price editing obey permission rules.
- Matrix selection is visual and keyboard navigable.
- Payment flow minimizes steps.
- Split payment clearly shows remaining amount.
- Receipt/print failures never roll back a committed sale.
- Offline status is persistent but unobtrusive.

## Internationalization

All user-facing strings are translation keys. Do not hardcode user-visible text in domain logic. Initial locales: English, Arabic, French. Support RTL switching without duplicating layouts.

Currency, number, date, time, tax and measurement formatting use locale-aware utilities.

## Accessibility

Keyboard navigation, visible focus, semantic controls, adequate contrast, screen-reader labels, reduced-motion preference, and error messages associated with inputs are required for core workflows.

## State design

Every network/cloud-dependent view must define loading, empty, error, offline, permission-denied and success states where applicable. Financial mutations must show deterministic pending/success/failure behavior and never imply success before the authoritative local transaction commits.
