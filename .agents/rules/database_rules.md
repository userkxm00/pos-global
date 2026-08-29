# Database & Migration Rules

> **Scope:** `pos-global` SQLite Database, Schema, and Migrations  
> **Status:** ACTIVE

---

## 1. Migration Standards

### 1.1 Sequential & Forward-Only
- All migrations live in `src-tauri/migrations/`.
- File naming convention: `NNN_description.sql` (e.g., `010_products.sql`, `011_categories_brands_manufacturers.sql`).
- **Never edit an existing applied migration.** If changes are needed, write the next sequential migration.

### 1.2 Foreign Key Constraints & Deferred Checks
- SQLite foreign keys are enforced via `PRAGMA foreign_keys = ON;`.
- In fixtures or cyclic graph situations (e.g. self-referencing category trees), insert parent nodes before child nodes or handle deferral correctly under transaction boundaries.
- Foreign key failure code (extended error 787) must be handled gracefully in application services.

### 1.3 Partial Unique Indexes for Soft-Deleted Records
- Tables with soft-delete columns (`deleted_at`, `is_active`) must use partial unique indexes to ensure uniqueness among active records while permitting re-use of names after deletion.

### 1.4 Migration Verification Protocol
Before any migration PR is merged:
1. Verify fresh database migration execution from migration `001` through latest.
2. Verify rollback/upgrade scripts if applicable.
3. Verify test fixtures run cleanly against the new schema.
