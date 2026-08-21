# POS Global — Industry Execution Plan

Industry work is a composition layer over shared capabilities. Each family must preserve the common sales, inventory, financial, authorization, sync and audit invariants.

## Universal industry sequence

For every industry family:

1. define preset and enabled capabilities;
2. define domain objects/events unique to the workflow;
3. define screens and commands;
4. define inventory/accounting impact;
5. define permissions;
6. define offline/idempotency behavior;
7. define reports/audit requirements;
8. add unit/integration/E2E tests;
9. update capability matrix and acceptance evidence.

## Retail / General Commerce — `F7.01.*`

- `F7.01.01` general retail preset
- `F7.01.02` catalog/search workflow
- `F7.01.03` promotions/basic price rules
- `F7.01.04` returns/exchanges workflow
- `F7.01.05` multi-location retail views
- `F7.01.06` acceptance/evidence

## Fashion / Shoes — `F7.02.*`

- `F7.02.01` fashion preset
- `F7.02.02` size/color/material attributes
- `F7.02.03` matrix grid operations
- `F7.02.04` seasonal/collection metadata
- `F7.02.05` variant-aware purchasing/returns
- `F7.02.06` acceptance/evidence

## Grocery / Convenience / Produce — `F7.03.*`

- `F7.03.01` grocery preset
- `F7.03.02` weighted/variable quantity workflow
- `F7.03.03` batch/expiry/FEFO workflow
- `F7.03.04` weighed-price validation
- `F7.03.05` waste/shrinkage workflow
- `F7.03.06` acceptance/evidence

## Electronics / Mobile / Appliances — `F7.04.*`

- `F7.04.01` electronics preset
- `F7.04.02` serial/IMEI intake and lookup
- `F7.04.03` warranty linkage
- `F7.04.04` asset/customer handoff
- `F7.04.05` return/DOA workflow
- `F7.04.06` acceptance/evidence

## Automotive — `F7.05.*`

- `F7.05.01` automotive preset
- `F7.05.02` vehicle/customer association
- `F7.05.03` parts/SKU compatibility metadata
- `F7.05.04` workshop/service order composition
- `F7.05.05` labor + parts billing
- `F7.05.06` acceptance/evidence

## Pharmacy / Medical Retail — `F7.06.*`

- `F7.06.01` pharmacy preset boundary
- `F7.06.02` batch/expiry/FEFO
- `F7.06.03` controlled-product capability gate
- `F7.06.04` prescription/regulated workflow adapter boundary
- `F7.06.05` jurisdiction-specific compliance checks
- `F7.06.06` acceptance/evidence

Production claims require country-specific regulatory approval; this module cannot be marketed as legally compliant from the generic core.

## Furniture / Home — `F7.07.*`

- `F7.07.01` furniture preset
- `F7.07.02` dimensions/material/options
- `F7.07.03` deposits/holds
- `F7.07.04` delivery/fulfillment status
- `F7.07.05` warranty/returns linkage
- `F7.07.06` acceptance/evidence

## Hardware / Building — `F7.08.*`

- `F7.08.01` hardware preset
- `F7.08.02` units/packaging/conversions
- `F7.08.03` cut-to-measure workflow
- `F7.08.04` B2B pricing/credit
- `F7.08.05` supplier/stock workflow
- `F7.08.06` acceptance/evidence

## Restaurant / Café / Fast Food — `F7.09.*`

- `F7.09.01` restaurant/café preset
- `F7.09.02` tables/sections/floor plan
- `F7.09.03` open orders and table transfer
- `F7.09.04` modifiers/options/notes
- `F7.09.05` order routing
- `F7.09.06` kitchen display system adapter
- `F7.09.07` courses/hold/fire workflow where enabled
- `F7.09.08` split/merge bills and service charges
- `F7.09.09` tips/gratuity policy adapter
- `F7.09.10` recipes/ingredient consumption
- `F7.09.11` waste/comps/void policy
- `F7.09.12` acceptance/evidence

## Bakery / Food Production — `F7.10.*`

- `F7.10.01` bakery preset
- `F7.10.02` recipes/BOM
- `F7.10.03` ingredient consumption
- `F7.10.04` batch production
- `F7.10.05` yield/waste tracking
- `F7.10.06` expiry/FEFO
- `F7.10.07` acceptance/evidence

## Repair / Service — `F7.11.*`

- `F7.11.01` service preset
- `F7.11.02` service ticket/intake
- `F7.11.03` customer asset/device
- `F7.11.04` diagnosis/status workflow
- `F7.11.05` parts + labor lines
- `F7.11.06` estimate → approval → work → completion
- `F7.11.07` warranty/return workflow
- `F7.11.08` acceptance/evidence

## Rental — `F7.12.*`

- `F7.12.01` rental preset
- `F7.12.02` asset inventory/condition
- `F7.12.03` availability calendar
- `F7.12.04` reservation/hold
- `F7.12.05` contract/check-out
- `F7.12.06` return/check-in/condition
- `F7.12.07` deposit/late fee policy adapter
- `F7.12.08` acceptance/evidence

## Salon / Beauty — `F7.13.*`

- `F7.13.01` salon preset
- `F7.13.02` services/catalog
- `F7.13.03` appointments/calendar
- `F7.13.04` staff/resource assignment
- `F7.13.05` product + service ticketing
- `F7.13.06` packages/memberships where enabled
- `F7.13.07` acceptance/evidence

## Wholesale / B2B — `F7.14.*`

- `F7.14.01` wholesale preset
- `F7.14.02` customer/company profiles
- `F7.14.03` price lists/tiers
- `F7.14.04` credit limits/terms
- `F7.14.05` quotations/orders
- `F7.14.06` delivery/invoice workflow
- `F7.14.07` acceptance/evidence

## Hospitality — `F7.15.*`

- `F7.15.01` hospitality preset
- `F7.15.02` guest/customer profile
- `F7.15.03` room/resource inventory abstraction
- `F7.15.04` reservation integration boundary
- `F7.15.05` charges/folios interface
- `F7.15.06` POS charge-posting adapter
- `F7.15.07` acceptance/evidence

## Events — `F7.16.*`

- `F7.16.01` events preset
- `F7.16.02` event/catalog setup
- `F7.16.03` ticket/registration inventory
- `F7.16.04` reservations/holds
- `F7.16.05` check-in/scan adapter
- `F7.16.06` refunds/cancellations
- `F7.16.07` acceptance/evidence

## Custom capability composer — `F7.17.*`

- `F7.17.01` capability selection model
- `F7.17.02` dependency validation
- `F7.17.03` incompatible-capability rules
- `F7.17.04` plan/jurisdiction enforcement
- `F7.17.05` onboarding preview
- `F7.17.06` acceptance/evidence

## Industry gate

No industry family is complete until its tasks prove that:

- shared financial truth remains exact;
- inventory mutations remain ledger-backed;
- permissions are enforced outside the UI;
- offline behavior is explicitly defined;
- retries are idempotent;
- tax behavior comes from jurisdiction contracts;
- regulated functionality is not marketed beyond verified scope;
- module code composes the shared core instead of duplicating it.
