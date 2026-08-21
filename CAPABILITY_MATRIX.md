# POS Global — Industry Capability Matrix

This matrix defines onboarding presets. A preset enables defaults; it does not create a separate codebase or hardcoded product type. Product behavior remains capability-driven and can be overridden within plan and jurisdiction limits.

Legend: `Core` = shared capability expected by the workflow; `Opt` = optional/configurable; `N/A` = not a primary workflow.

| Capability | Retail | Fashion | Grocery | Electronics | Automotive | Pharmacy | Furniture | Hardware | Restaurant/Café | Bakery/Food | Repair/Service | Rental | Salon/Beauty | Wholesale/B2B | Hospitality | Events | Custom |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| SKU/barcode | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Opt | Core |
| Matrix variants | Opt | Core | Opt | Opt | N/A | N/A | Opt | Opt | Opt | Opt | N/A | Opt | Opt | Opt | Opt | Opt | Opt |
| Weight/measure | Opt | N/A | Core | Opt | Opt | Opt | Opt | Opt | Core | Core | Opt | Opt | Opt | Opt | Opt | Opt | Opt |
| Batch/lot | Opt | Opt | Core | Opt | Opt | Core | Opt | Opt | Opt | Core | Opt | Opt | Opt | Opt | Opt | Opt | Opt |
| Expiry/FEFO | Opt | N/A | Core | N/A | Opt | Core | N/A | Opt | Opt | Core | Opt | N/A | Opt | Opt | Opt | Opt | Opt |
| Serial/IMEI | Opt | N/A | N/A | Core | Core | Opt | Opt | Opt | N/A | N/A | Core | Opt | N/A | Opt | Opt | Opt | Opt |
| Warranty | Opt | Opt | N/A | Core | Core | Opt | Core | Core | N/A | N/A | Core | Opt | Opt | Opt | Core | Opt | Opt |
| Recipes/BOM | Opt | N/A | Opt | N/A | N/A | N/A | Opt | N/A | Core | Core | Opt | N/A | N/A | Opt | Core | Opt | Opt |
| Tables/orders | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | Core | Core | N/A | N/A | N/A | N/A | Core | Opt | Opt |
| KDS/kitchen routing | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | Core | Core | N/A | N/A | N/A | N/A | Core | Opt | Opt |
| Service tickets/labor | Opt | N/A | Opt | Opt | Core | Opt | Opt | Opt | Opt | Opt | Core | N/A | Opt | Opt | Opt | Opt | Opt |
| Rental assets/contracts | N/A | Opt | N/A | Opt | Core | N/A | Opt | Opt | N/A | N/A | Opt | Core | N/A | Opt | Opt | Opt | Opt |
| Appointments | N/A | Opt | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | Core | Opt | Core | N/A | Core | Opt | Opt |
| Reservations | N/A | N/A | N/A | N/A | Opt | N/A | Opt | N/A | Opt | Opt | N/A | Core | Core | Opt | Core | Core | Opt |
| B2B pricing/credit | Opt | Opt | Opt | Opt | Core | Opt | Opt | Core | N/A | N/A | Opt | Opt | N/A | Core | Core | Opt | Core |
| Deposits/holds | Opt | Opt | Opt | Opt | Core | Opt | Core | Opt | Core | Core | Opt | Core | Opt | Core | Core | Core | Opt |
| Multi-location stock | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core |
| Tax/jurisdiction adapter | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core |
| Audit/ledger invariants | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core | Core |

## Preset rules

1. Presets are onboarding defaults only.
2. A business may use multiple capabilities across one catalog.
3. A product may combine capabilities regardless of industry preset.
4. Regulated capabilities require jurisdiction policy before production use.
5. Shared sales, inventory, cash, debt, tax and authorization invariants are never forked per industry.
6. The UI should expose only enabled capabilities by default, while advanced settings can reveal additional allowed capabilities.
7. Every capability that creates data or changes transaction semantics must have a corresponding task ID, domain contract and acceptance tests.

## Capability-to-phase rule

Capabilities are introduced in shared core phases first (Identity, Inventory, Sales, Purchasing, Sync), then industry workflow phases compose them. Industry tasks must not reimplement shared accounting, stock or authorization rules.
