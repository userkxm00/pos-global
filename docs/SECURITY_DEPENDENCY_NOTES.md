# Security dependency notes

## RUSTSEC-2024-0429 — glib VariantStrIter

Status: upstream-blocked mitigation.

The current Tauri v2 Linux WebView dependency stack transitively resolves `glib` 0.18.x through GTK3-era crates. RustSec marks `glib >=0.15.0,<0.20.0` as affected and `>=0.20.0` as patched.

We must not force `glib 0.20` with a manual Cargo patch because the current GTK/Tauri dependency graph requires the 0.18 series and a forced major-version substitution can create an incompatible mixed GTK stack.

The project currently does not reference `glib::VariantStrIter` directly. The issue is therefore tracked as a transitive Linux-stack advisory, not falsely marked as fixed.

Exit condition:
- adopt an upstream Tauri/Wry Linux stack that resolves to `glib >=0.20`,
- regenerate `Cargo.lock`,
- run full Linux build/test/Clippy/CodeQL/Sonar validation,
- then remove this note and close the tracking issue.
