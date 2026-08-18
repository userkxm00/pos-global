# POS Global — Phase 0.6: Commercial & Regulatory Finalization

Status: REQUIRED BEFORE AUTONOMOUS COMMERCIAL IMPLEMENTATION
Purpose: define what is product-core policy, what is provider-specific, and what must be verified per jurisdiction before launch.

## 1. Product business model

### 1.1 License architecture

Zylo/POS Global is a commercial desktop product with optional cloud services. The license system must support:
- trial
- paid subscription
- annual billing
- plan entitlements
- branch/device/user limits
- feature entitlements
- offline grace period
- activation/deactivation
- device reset
- revocation
- plan upgrade/downgrade

The exact public pricing is a product-owner decision and must not be hardcoded into the POS core.

### 1.2 Entitlement boundary

The desktop client receives signed entitlements such as:
- plan_id
- organization_id
- allowed_features
- max_branches
- max_devices
- max_users
- expiration/grace data
- license_id

The client must not be trusted as the source of truth for billing. The cloud licensing service is authoritative when online; the signed license provides offline continuity.

## 2. Billing provider strategy

### Decision: provider-neutral billing boundary

Do not couple the application to Stripe/Paddle/etc. The website talks to a `BillingProvider` interface. Webhooks are translated into our internal events:
- subscription_created
- subscription_updated
- subscription_paused
- subscription_cancelled
- payment_succeeded
- payment_failed
- refund
- chargeback/dispute where applicable

### Candidate: Paddle for SaaS/software subscriptions

Paddle is a strong candidate for the software-license website because it operates as Merchant of Record for digital products and states that it handles sales tax/VAT collection, filing and remittance across supported jurisdictions. This can reduce the amount of global digital-sales tax infrastructure we must build ourselves.

This is a recommendation, not a final legal decision. Before production selection, verify:
- company eligibility;
- supported seller country/entity;
- product eligibility;
- payout availability;
- refund/chargeback rules;
- invoice requirements;
- license fulfillment workflow;
- webhook guarantees;
- pricing/fees;
- data-processing/privacy terms.

### Stripe candidate

Stripe is a strong provider for direct payment infrastructure where the merchant entity is in a supported country. Stripe's current global availability list does not list Algeria as a directly supported business country, while France and many EU countries are supported. Therefore the POS architecture must not assume Stripe is available to an Algerian merchant entity.

If a future company structure is eligible for Stripe, use it behind the same billing/payment adapter rather than coupling the product to it.

## 3. POS payment strategy

POS payments are separate from SaaS subscription billing.

### Core payment abstraction

`PaymentMethod -> PaymentProcessor -> Provider/Terminal Adapter`

Initial methods:
- cash
- card terminal
- bank transfer
- customer credit
- manual/other
- online payment where a provider is legally/technically supported

The core POS transaction remains provider-independent.

### Provider selection rule

For each target market, select payment integrations based on:
- local merchant availability
- terminal APIs/SDKs
- offline behavior
- settlement
- refund support
- idempotency
- reconciliation
- local payment methods
- fees
- regulatory requirements

## 4. Tax architecture

The POS tax engine and the SaaS website billing tax are distinct systems.

### POS tax

The POS must calculate/record the merchant's own transaction tax according to the configured jurisdiction and business regime.

### SaaS billing tax

The website billing provider may act as Merchant of Record and handle taxes for the software sale where supported. The commercial/legal team must confirm the seller-of-record relationship before launch.

## 5. Algeria launch research package

The Algerian Directorate General of Taxes currently documents VAT under the real/simplified regimes, including a normal 19% rate and reduced 9% rate, with different taxable-event rules for goods and services. This must be encoded as jurisdiction data/rules rather than as a global constant.

Required Algeria adapter research before production:
- taxpayer regime (including IFU treatment where relevant)
- VAT registration status
- invoice mandatory fields
- numbering/sequence rules
- cash receipt requirements
- returns/credit notes
- tax exemptions
- product/service classification
- e-invoicing/fiscalization requirements applicable at launch
- accounting export requirements
- retention/audit requirements

The official DGI source must be the primary reference and the implementation must record source date/version in regulatory metadata.

## 6. France/EU expansion research package

The European Commission states that EU VAT operates under a common framework while Member States set their own rates and categories within that framework. Therefore the product must use country-specific jurisdiction configuration, not a single EU VAT rate.

France adapter research must verify:
- standard/reduced rates and product categories;
- B2B/B2C treatment;
- VAT ID validation requirements;
- invoice mandatory fields;
- exemptions/reverse charge;
- distance/e-commerce implications where applicable;
- electronic invoicing timetable and scope;
- retention/audit rules;
- cash/register fiscal requirements applicable to the target business type.

Do not claim EU/France tax compliance until the current official rules and applicable merchant regime are verified.

## 7. Regulatory adapter architecture

Create a jurisdiction package with:
- country_code
- region/state where applicable
- tax rules
- invoice rules
- receipt rules
- numbering policy
- retention policy
- required customer/business identifiers
- fiscalization adapter where legally required
- effective dates
- authoritative source references
- review date

The core POS engine consumes capabilities from the jurisdiction package. Regulatory code must not be scattered through sales/product UI code.

## 8. Industry regulatory tiers

Not every industry is equally regulated.

### Tier A — general retail
Retail, fashion, furniture, hardware, many services.

### Tier B — operationally specialized
Grocery, restaurant, café, bakery, automotive, repair, rental, hospitality, events.

### Tier C — highly regulated
Pharmacy/medical retail and any jurisdiction-specific controlled-product workflows.

Tier C modules must not be marketed as legally compliant until jurisdiction-specific legal/technical requirements are researched and verified.

## 9. Commercial website

The website must support:
- product overview
- industries/capabilities
- pricing
- comparison
- trial
- checkout
- account portal
- download
- license activation
- device management
- invoices
- subscription management
- support
- documentation
- release notes/status

The website must never contain private desktop signing keys or Supabase secret/service-role keys.

## 10. Customer lifecycle

`Visitor -> Trial -> Account -> Checkout -> Subscription -> License -> Activation -> Renewal -> Upgrade/Downgrade -> Cancellation/Grace -> Reactivation`

Every transition must be represented as an auditable cloud event.

## 11. Launch-market policy

Default planning recommendation:
1. Algeria as the first local validation market.
2. France as the first international validation market.
3. Broader EU expansion after France-specific validation and reusable EU jurisdiction architecture.

This is a planning recommendation, not a legal conclusion. The product owner must approve the launch sequence before production claims are made.

## 12. Regulatory evidence policy

For every supported launch jurisdiction maintain:
- `REGULATORY/<country>/README.md`
- authoritative source URLs
- source access date
- rule version/effective date
- implementation mapping
- test cases
- open legal questions
- owner/review date

The agent must never cite a blog or vendor article as the sole authority for a legal requirement when an official source exists.

## 13. Exit gate

Phase 0.6 is complete only when:
- license model is frozen;
- billing abstraction is frozen;
- SaaS billing provider candidate has an eligibility/legal checklist;
- POS payment abstraction is frozen;
- launch markets are explicitly approved;
- Algeria regulatory package is researched to implementation-ready level;
- France/EU package has an explicit scope and unresolved-items list;
- regulatory adapter architecture is frozen;
- website commercial lifecycle is frozen;
- no global compliance claim is made without jurisdiction evidence.
