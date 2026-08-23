# Upstream-blocked glib advisory

This note tracks RUSTSEC-2024-0429 affecting the GTK3-era Linux dependency path inherited from Tauri v2.

Do not force a `glib 0.20` patch into the existing GTK3 dependency graph. The current stack requires compatible 0.18-series bindings; forcing a major-version substitution would risk an incompatible GTK/GIO/GDK stack.

Exit condition: move to an upstream Tauri/Wry Linux stack that resolves `glib >=0.20`, regenerate Cargo.lock, and pass the complete CI/security matrix.