# Phase Memory Index

> **Timeline of Completed Phases, Architectural Invariants, and Hand-Off Memory**  
> **Storage Path:** `.agents/memory/phases/`

---

## Phase Memory Registry

| Phase ID | Milestone Name | Status | Key Deliverables | Protected Files |
| :--- | :--- | :--- | :--- | :--- |
| **[PHASE-F1](./PHASE-F1.md)** | Foundation (F1.01–F1.25) | MERGED / PROTECTED | SQLite DB, Migrations 001–009, Auth, Audit Log, Tauri IPC | `src-tauri/src/auth/`, `src-tauri/migrations/001-009` |
| **[PHASE-F201](./PHASE-F201.md)** | F2.01 Products CRUD | MERGED / PROTECTED | Migration 010, Product CRUD, Barcodes, SKU uniqueness | `src-tauri/src/product/`, `src-tauri/migrations/010_products.sql` |
| **[PHASE-F202](./PHASE-F202.md)** | F2.02 Categories, Brands, Manufacturers | MERGED / PROTECTED | Migration 011, Taxonomy Tree, URL Validation, Sonar 0 issues | `src-tauri/src/category/`, `src-tauri/src/brand/`, `src-tauri/src/manufacturer/` |
