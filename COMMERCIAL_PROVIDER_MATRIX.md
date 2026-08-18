# Commercial Provider Decision Matrix

Status: ARCHITECTURE DECISION SUPPORT — NOT FINAL CONTRACTS
Last reviewed: 2026-08-17

## 1. SaaS subscription billing

| Provider | Role | Strength | Current evidence/constraint | Decision |
|---|---|---|---|---|
| Paddle | Merchant of Record / SaaS billing | Digital-product billing, subscriptions, checkout and tax/compliance handling | Paddle's current supported-country documentation does not list Algeria among unsupported supplier countries, and its documentation explicitly includes Algeria in supported country data. Seller/entity onboarding and product eligibility still require final confirmation. | **Primary candidate** |
| Stripe | Payment/billing infrastructure | Mature APIs, broad payment methods, strong ecosystem | Stripe's current supported-business-country list does not include Algeria, while France and many EU countries are supported. Eligibility depends on the actual merchant entity. | **Secondary adapter candidate** |

Paddle publicly describes itself as Merchant of Record for digital products and states that it calculates, collects and remits taxes for supported transactions. Stripe publicly lists its supported business countries/regions and currently does not list Algeria as a directly supported business country.

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

For Zylo's own website subscription billing, investigate Paddle first because its Merchant-of-Record model can reduce the global digital-tax/compliance surface. Confirm seller eligibility, fees, payout/banking and legal terms before production.

For POS customer payments, implement cash and manual/credit flows first, then add card-terminal/payment-provider adapters per launch market. Do not assume Stripe availability for an Algerian merchant entity.
