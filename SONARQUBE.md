# SonarQube Cloud verification

GitHub Actions automatic verification is enabled because this repository is public and uses GitHub-hosted standard runners. Gitar and SonarQube Cloud remain complementary review/analysis layers.

## SonarQube Cloud

The repository is configured with `sonar-project.properties` for the `userkxm00_pos-global` project key and `userkxm00` organization. A SonarQube Cloud project must still be imported/connected in SonarQube Cloud; the properties file alone does not create the cloud project.

The CI workflow runs the SonarQube scan on pushes to `main`, manual verification runs, and same-repository pull requests when the required `SONAR_TOKEN` secret is available. Pull requests from forks do not receive the secret and therefore do not run the Sonar job.

Important: Rust is currently not eligible for SonarQube Cloud Automatic Analysis. Sonar is therefore a supplemental static-analysis signal for supported code, not a replacement for Rust formatting, clippy, or tests. Gitar provides an additional cross-language review signal.

When GitHub Actions is later moved back to a private repository or a quota-constrained runner environment, preserve the same review separation: Gitar for contextual code review, SonarQube for static analysis, and executable Rust/frontend tests for behavior verification.
