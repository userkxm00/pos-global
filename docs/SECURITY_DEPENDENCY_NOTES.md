# Security dependency notes

## RUSTSEC-2024-0429 — glib VariantStrIter

**Status:** upstream-blocked mitigation.

The current Tauri v2 Linux WebView dependency stack transitively resolves `glib` 0.18.x through GTK3-era crates. RustSec marks `glib >=0.15.0,<0.20.0` as affected and `>=0.20.0` as patched.

We intentionally do **not** force `glib 0.20` with a Cargo patch. The current GTK/Tauri dependency graph requires mutually compatible 0.18-series bindings; a forced major-version substitution could create an incompatible GTK/GIO/GDK/WebKit stack and would be less safe than the advisory itself.

The application source does not reference `glib::VariantStrIter` directly. This is tracked as a transitive Linux-stack advisory while upstream completes the GTK4 migration.

### Exit condition

1. Adopt an upstream Tauri/Wry Linux stack that resolves `glib >=0.20`.
2. Regenerate `src-tauri/Cargo.lock`.
3. Run the full Linux build, Clippy, tests, CodeQL, Secret Scan, and SonarQube checks.
4. Remove this note and close the advisory once the dependency graph is clean.
