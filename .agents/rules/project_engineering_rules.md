# POS Global — Project Engineering Rules

> **Scope:** `pos-global` Repository  
> **Status:** ACTIVE  
> **Stack:** Tauri v2, Rust (2021 edition), SQLite, React, TypeScript, Tailwind CSS

---

## 1. Architecture & Tenancy Boundaries

### 1.1 Single-Tenant Local SQLite
- The application runs as a local desktop POS with single-tenant SQLite database per installation.
- All IDs use UUIDv4 strings.
- All timestamps use ISO-8601 UTC strings (`datetime('now')`).

### 1.2 Soft-Deletion Invariants
- Entities use `is_active` (INTEGER 0/1) and `deleted_at` (TEXT ISO timestamp).
- Queries default to filtering active, non-deleted records unless explicitly querying audit/trash views.
- Unique constraints must use partial unique indexes to account for soft-deleted duplicates:
  ```sql
  CREATE UNIQUE INDEX idx_categories_name_parent_unique 
  ON categories(tenant_id, parent_id, name) 
  WHERE deleted_at IS NULL AND is_active = 1;
  ```

### 1.3 Money & Financial Precision
- Financial amounts must be stored as integer cents/halalas (`INTEGER` in SQLite, `i64` in Rust) or precise decimal types to prevent floating-point rounding errors.

### 1.4 Phase Isolation & Protection
- Merged phases (e.g., `F1.*` foundation, `F2.01` products CRUD) are protected against regressions.
- Future phase responsibilities (e.g. `F2.17` product-brand relations, `F2.05` variants) must NOT be implemented prematurely.

---

## 2. Rust Backend Standards

### 2.1 Error Handling
- Use strongly-typed `thiserror` domain enums for each module (`CategoryError`, `ProductError`, `BrandError`, etc.).
- SQLite constraint errors must be explicitly mapped to domain errors (e.g., `SqliteFailure` with extended code 787 mapped to `ForeignKeyViolation` or `DuplicateName`).
- Never panic in production code; return `Result<T, DomainError>`.

### 2.2 Tauri Commands & IPC
- Tauri commands in `src-tauri/src/commands/` must validate inputs, delegate to service/repository layers, and return serialized JSON results.
- Keep command handlers thin.

---

## 3. Frontend Standards

### 3.1 Type Safety & API Synchronization
- TypeScript interfaces in `src/types/` must match Rust DTOs and database schemas exactly.
- All API calls invoke Tauri commands via typed wrappers.

### 3.2 UI Design & Accessibility
- Bilingual support (Arabic RTL and English LTR).
- Keyboard-first POS shortcuts for cashiers.
