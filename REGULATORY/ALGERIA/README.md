# Algeria Regulatory Research Baseline

Status: RESEARCH BASELINE — NOT A LEGAL COMPLIANCE CERTIFICATION
Last reviewed: 2026-08-17

## Primary source

Algeria Directorate General of Taxes (DGI): VAT guidance for the real regime, updated 23 February 2026:
https://www.mfdgi.gov.dz/fr/professionnels/services-pro/regime-reel/la-taxe-sur-la-valeur-ajoutee

## Verified baseline

The DGI page states:
- normal VAT rate: 19%
- reduced VAT rate: 9%
- VAT treatment differs by taxable event and type of operation;
- sales of goods generally use delivery as the taxable event, while many services use partial/full collection;
- IFU taxpayers are described separately from the VAT regimes.

These values are evidence for the Algeria jurisdiction package only. They must never be copied into generic POS code.

## Implementation requirements

The Algeria adapter must model at least:
- tax regime
- VAT registration status
- normal/reduced rates
- product/service tax classification
- taxable event
- tax-inclusive/exclusive presentation
- exemptions
- customer/business identifiers
- invoice numbering and required invoice fields
- credit notes/refunds
- reporting/export requirements
- retention/audit metadata

## Open research before production

The following require a fresh official-source review before any compliance claim:
- current invoicing requirements and mandatory fields;
- fiscalization/e-invoicing requirements applicable to the target merchant types;
- receipt/cash-register obligations;
- retention periods;
- sector-specific rules for pharmacy, food, hospitality and other regulated industries;
- accounting/export formats;
- treatment of B2B vs B2C transactions;
- rules for exemptions and special regimes;
- any 2026 Finance Law changes affecting POS obligations.

## Rule

The implementation agent must not declare `Algeria compliant` based only on this file. A launch readiness review must link each implemented rule to an authoritative source and test case.
