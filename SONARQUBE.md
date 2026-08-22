# SonarQube Cloud verification

GitHub-hosted Actions are intentionally not an automatic gate while Actions minutes are exhausted.

For ongoing review, use Gitar plus SonarQube Cloud.

## SonarQube Cloud

Prefer SonarQube Cloud Automatic Analysis for the GitHub repository when the project is eligible. Automatic Analysis runs in SonarQube Cloud and therefore does not consume GitHub Actions minutes.

Important: Rust is currently not eligible for SonarQube Cloud Automatic Analysis. Sonar is therefore a supplemental static-analysis signal for supported code (not a replacement for Rust tests). Gitar remains the primary cross-language review during the Actions-quota constraint.

If CI-based Sonar analysis is required later for Rust/full-project analysis, re-enable the GitHub Actions Sonar workflow after Actions capacity is available or move it to a suitable runner.
