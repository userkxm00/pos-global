# PRODUCT SPECIFICATION

## Product vision
A global desktop commerce operating platform for shops and service businesses. It must feel simple for a cashier while exposing powerful management capabilities to owners and managers.

## Primary user roles

- Owner/Admin
- Manager
- Cashier
- Inventory/Stock operator
- Purchaser
- Accountant/finance operator
- Service/repair operator
- Restaurant operator
- Auditor/read-only

Roles are composed from permissions and may be customized.

## Core navigation

1. Dashboard
2. POS / Sales
3. Products
4. Inventory
5. Purchases
6. Customers
7. Suppliers
8. Cash / Shifts
9. Reports
10. Staff / Permissions
11. Branches / Registers
12. Settings
13. Sync / Device status
14. License / Account

Visible navigation is permission- and capability-aware.

## Critical user journeys

### First setup
Create account → create organization → choose business preset → configure currency/tax basics → create branch/register → create first user → optional sample/import → ready for sale.

### Daily sale
Login/PIN → open shift → scan/search products → matrix/variant selection if needed → discounts/tax → customer optional → payment/split payment → receipt → stock/cash update.

### Offline sale
Continue the same workflow while disconnected → local transaction commits → visible offline status → reconnect → synchronize → reconcile cloud status.

### Purchase
Supplier → purchase order/receive → verify quantities/cost → inventory movement → supplier balance/payment → audit.

### Refund
Find sale → choose refundable lines → authorize → refund payment/cash/debt → reverse stock where applicable → audit.

### Inventory count
Select location → count → compare expected → approve adjustment → ledger movement → audit.

## Global requirements

- multilingual and RTL-ready;
- multiple currencies and locale formatting;
- tax configuration must be jurisdiction-aware and provider-neutral;
- keyboard-first POS operation;
- barcode-first workflows;
- accessible error/loading/empty states;
- offline visibility;
- import/export with validation and preview;
- backups and restore tooling;
- audit history for sensitive actions.

## Industry capability examples

Fashion: matrix, size/color, seasonal pricing.

Grocery: weight, units, batches, expiry/FEFO, barcode scales.

Electronics: serial/IMEI, warranty, model/manufacturer.

Automotive: part number, compatibility, VIN/job association where applicable.

Pharmacy/medical retail: batch/expiry and jurisdiction-specific controls; never assume universal regulatory rules.

Restaurant/café: menu items, modifiers, recipes/ingredients, table/order workflow, kitchen routing where enabled.

Repair/service: jobs, assets, statuses, parts/labor, deposits, estimates, completion and pickup.

Rental: assets, availability, reservations, deposits, check-out/check-in, late/damage charges.

Wholesale: customer groups, price lists, minimum quantities, terms, delivery documents.

Hospitality/events: reservations/orders/tickets and configurable service workflows.

These are capability compositions, not separate databases/products.
