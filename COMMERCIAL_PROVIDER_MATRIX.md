# Commercial Provider Decision Matrix

Status: ARCHITECTURE DECISION SUPPORT — NOT FINAL CONTRACTS
Last reviewed: 2026-08-17

## 1. SaaS subscription billing

| Provider | Role | Strength | Important constraint | Decision |
|---|---|---|---|---|
| Paddle | Merchant of Record / SaaS billing | Global digital-product tax, billing, checkout, subscriptions | Seller eligibility, product eligibility, fees and exact availability must be verified for our company/entity | **Primary candidate** |
| Stripe | Payment/billing infrastructure | Mature APIs, broad payment methods, strong ecosystem | Current supported-business-country list does not include Algeria; merchant entity eligibility matters | **Secondary candidate / provider adapter** |

Paddle publicly describes itself as Merchant of Record for digital products and says it handles tax collection/remittance across supported jurisdictions. Stripe's public global availability list currently includes France and many other countries but not Algeria as a directly supported business country.

## 2. Architectural rule

Do not expose provider-specific models to POS core.

Use:

`BillingProvider -> internal BillingEvent`

`PaymentProvider -> internal PaymentResult`

The internal model owns:
- order_id
- customer_id
- organization_id
- amount/currency
- provider_reference
- idempotency_key
- status
- timestamps
- raw provider event reference

Provider webhooks must be verified, idempotently processed and converted into internal events.

## 3. Selection gates

Before production selection, verify for the actual legal entity:
- seller country support;
- bank/payout support;
- product eligibility;
- KYC requirements;
- recurring billing support;
- refunds;
- chargebacks/disputes;
- tax handling;
- invoice handling;
- webhook delivery/retry semantics;
- data processing/privacy terms;
- API/SDK maturity;
- total cost.

## 4. Important separation

SaaS billing is not the same as merchant POS payments.

A customer buying a Zylo subscription on the website is a digital-product billing transaction.

A Zylo customer using the POS to accept a shopper's card payment is a merchant-payment transaction belonging to the customer's business.

These must remain separate abstractions and separate provider decisions.

## 5. Current recommendation

Build the architecture provider-neutral now.

For Zylo's own website subscription billing, investigate Paddle first because its Merchant-of-Record model can reduce the global digital-tax/compliance surface. Confirm eligibility and legal terms before production.

For POS customer payments, implement cash and manual/credit flows first, then add card-terminal/payment-provider adapters per launch market. Do not assume Stripe availability for an Algerian merchant entity.
