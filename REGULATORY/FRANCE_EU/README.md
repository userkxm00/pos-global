# France / EU Regulatory Research Baseline

Status: RESEARCH BASELINE — NOT A LEGAL COMPLIANCE CERTIFICATION
Last reviewed: 2026-08-17

## Primary sources

European Commission VAT rates:
https://taxation-customs.ec.europa.eu/taxation/vat/vat-directive/vat-rates_en

French tax administration overview:
https://www.impots.gouv.fr/international-professionnel/fiscalite-des-entreprises

French BOFiP VAT rate guidance:
https://bofip.impots.gouv.fr/bofip/1380-PGP.html/identifiant%3DBOI-TVA-LIQ-10-20250514

## Verified baseline

The European Commission states that the EU VAT Directive provides the common framework while Member States set their rates and categories within that framework.

French official tax guidance currently documents:
- normal VAT rate: 20%
- reduced rates including 10% and 5.5%
- a special 2.1% rate for specified categories.

French rules contain many product/service-specific exceptions, so a single rate table is not sufficient.

## Implementation requirements

The France/EU jurisdiction model must support:
- country and territorial scope;
- VAT registration status;
- B2B/B2C classification;
- VAT ID/customer identifiers;
- product/service tax classification;
- place-of-supply logic where relevant;
- domestic/reverse-charge/exemption treatment;
- tax-inclusive/exclusive pricing;
- effective dates;
- invoice/credit-note requirements;
- electronic invoicing/fiscalization requirements where applicable;
- audit/retention metadata.

## Expansion rule

Do not treat `EU` as one tax jurisdiction. Each Member State is a separate jurisdiction adapter/configuration with a shared EU framework layer.

## Open research before production

For France specifically, verify the current rules applicable to the exact merchant types targeted by the launch:
- B2B/B2C invoicing;
- VAT ID validation;
- electronic invoicing timetable and scope;
- cash/register obligations;
- exemptions and reverse charge;
- food/restaurant/hospitality rates;
- pharmacy/medical rules;
- retention/audit requirements.

For each additional EU country, create a country-specific research package before enabling regulated tax/invoicing claims.

## Rule

The implementation agent must not declare `France compliant` or `EU compliant` without current authoritative evidence and jurisdiction-specific acceptance tests.
