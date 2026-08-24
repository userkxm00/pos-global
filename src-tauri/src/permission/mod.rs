// Roles and Permissions domain model and evaluation engine.
// F1.06 — Roles and Permissions

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Machine-readable authoritative permission catalog matching migration 004 seed data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    #[serde(rename = "sales.create")]
    SalesCreate,
    #[serde(rename = "sales.refund")]
    SalesRefund,
    #[serde(rename = "sales.void")]
    SalesVoid,
    #[serde(rename = "inventory.adjust")]
    InventoryAdjust,
    #[serde(rename = "inventory.transfer")]
    InventoryTransfer,
    #[serde(rename = "products.manage")]
    ProductsManage,
    #[serde(rename = "purchases.manage")]
    PurchasesManage,
    #[serde(rename = "customers.manage")]
    CustomersManage,
    #[serde(rename = "debts.manage")]
    DebtsManage,
    #[serde(rename = "cash.open")]
    CashOpen,
    #[serde(rename = "cash.close")]
    CashClose,
    #[serde(rename = "cash.adjust")]
    CashAdjust,
    #[serde(rename = "reports.view")]
    ReportsView,
    #[serde(rename = "reports.export")]
    ReportsExport,
    #[serde(rename = "users.manage")]
    UsersManage,
    #[serde(rename = "settings.manage")]
    SettingsManage,
    #[serde(rename = "license.manage")]
    LicenseManage,
}

impl Permission {
    /// Complete authoritative list of all system permissions.
    pub const ALL: &'static [Permission] = &[
        Permission::SalesCreate,
        Permission::SalesRefund,
        Permission::SalesVoid,
        Permission::InventoryAdjust,
        Permission::InventoryTransfer,
        Permission::ProductsManage,
        Permission::PurchasesManage,
        Permission::CustomersManage,
        Permission::DebtsManage,
        Permission::CashOpen,
        Permission::CashClose,
        Permission::CashAdjust,
        Permission::ReportsView,
        Permission::ReportsExport,
        Permission::UsersManage,
        Permission::SettingsManage,
        Permission::LicenseManage,
    ];

    /// Returns the exact string identifier for this permission.
    pub fn as_str(&self) -> &'static str {
        match self {
            Permission::SalesCreate => "sales.create",
            Permission::SalesRefund => "sales.refund",
            Permission::SalesVoid => "sales.void",
            Permission::InventoryAdjust => "inventory.adjust",
            Permission::InventoryTransfer => "inventory.transfer",
            Permission::ProductsManage => "products.manage",
            Permission::PurchasesManage => "purchases.manage",
            Permission::CustomersManage => "customers.manage",
            Permission::DebtsManage => "debts.manage",
            Permission::CashOpen => "cash.open",
            Permission::CashClose => "cash.close",
            Permission::CashAdjust => "cash.adjust",
            Permission::ReportsView => "reports.view",
            Permission::ReportsExport => "reports.export",
            Permission::UsersManage => "users.manage",
            Permission::SettingsManage => "settings.manage",
            Permission::LicenseManage => "license.manage",
        }
    }

    /// Parses an exact permission string into a typed Permission enum.
    /// Strict exact matching: fails closed on unknown values, casing, or whitespace.
    pub fn parse(s: &str) -> Option<Permission> {
        match s {
            "sales.create" => Some(Permission::SalesCreate),
            "sales.refund" => Some(Permission::SalesRefund),
            "sales.void" => Some(Permission::SalesVoid),
            "inventory.adjust" => Some(Permission::InventoryAdjust),
            "inventory.transfer" => Some(Permission::InventoryTransfer),
            "products.manage" => Some(Permission::ProductsManage),
            "purchases.manage" => Some(Permission::PurchasesManage),
            "customers.manage" => Some(Permission::CustomersManage),
            "debts.manage" => Some(Permission::DebtsManage),
            "cash.open" => Some(Permission::CashOpen),
            "cash.close" => Some(Permission::CashClose),
            "cash.adjust" => Some(Permission::CashAdjust),
            "reports.view" => Some(Permission::ReportsView),
            "reports.export" => Some(Permission::ReportsExport),
            "users.manage" => Some(Permission::UsersManage),
            "settings.manage" => Some(Permission::SettingsManage),
            "license.manage" => Some(Permission::LicenseManage),
            _ => None,
        }
    }
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Authoritative built-in roles supported by the domain model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "manager")]
    Manager,
    #[serde(rename = "cashier")]
    Cashier,
}

impl Role {
    /// Complete list of all supported built-in roles.
    #[allow(dead_code)]
    pub const ALL: &'static [Role] = &[Role::Admin, Role::Manager, Role::Cashier];

    /// Returns the exact string identifier for this role.
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Manager => "manager",
            Role::Cashier => "cashier",
        }
    }

    /// Parses an exact role string into a typed Role enum.
    /// Strict exact matching: fails closed on unknown values, casing, or whitespace.
    pub fn parse(s: &str) -> Option<Role> {
        match s {
            "admin" => Some(Role::Admin),
            "manager" => Some(Role::Manager),
            "cashier" => Some(Role::Cashier),
            _ => None,
        }
    }

    /// Returns default built-in permission set matching migration 004 seed rules.
    pub fn default_permissions(&self) -> &'static [Permission] {
        match self {
            Role::Admin => Permission::ALL,
            Role::Manager => &[
                Permission::SalesCreate,
                Permission::SalesRefund,
                Permission::SalesVoid,
                Permission::InventoryAdjust,
                Permission::InventoryTransfer,
                Permission::ProductsManage,
                Permission::PurchasesManage,
                Permission::CustomersManage,
                Permission::DebtsManage,
                Permission::CashOpen,
                Permission::CashClose,
                Permission::CashAdjust,
                Permission::ReportsView,
                Permission::ReportsExport,
                Permission::SettingsManage,
            ],
            Role::Cashier => &[
                Permission::SalesCreate,
                Permission::CustomersManage,
                Permission::ReportsView,
                Permission::CashOpen,
                Permission::CashClose,
            ],
        }
    }

    /// Checks if this role has the permission by default in the catalog.
    pub fn has_default_permission(&self, perm: Permission) -> bool {
        self.default_permissions().contains(&perm)
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Typed authorization errors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PermissionError {
    #[allow(dead_code)]
    PermissionDenied {
        role: String,
        permission: String,
        reason: String,
    },
    ScopeMismatch {
        expected: String,
        actual: String,
    },
    InvalidPermission(String),
    #[allow(dead_code)]
    InvalidRole(String),
    Database(String),
}

impl std::fmt::Display for PermissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermissionError::PermissionDenied {
                role,
                permission,
                reason,
            } => {
                write!(
                    f,
                    "Permission denied: role '{role}' lacks permission '{permission}'. Reason: {reason}"
                )
            }
            PermissionError::ScopeMismatch { expected, actual } => {
                write!(
                    f,
                    "Scope mismatch: operation expects scope '{expected}', but session has scope '{actual}'"
                )
            }
            PermissionError::InvalidPermission(p) => write!(f, "Invalid permission: '{p}'"),
            PermissionError::InvalidRole(r) => write!(f, "Invalid role: '{r}'"),
            PermissionError::Database(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

impl std::error::Error for PermissionError {}

/// In-memory fast role permission check with deny-by-default semantics.
/// Fails closed on unknown roles or invalid permissions.
#[allow(dead_code)]
pub fn check_role_permission(role_str: &str, perm_str: &str) -> bool {
    let role = match Role::parse(role_str) {
        Some(r) => r,
        None => return false,
    };
    let perm = match Permission::parse(perm_str) {
        Some(p) => p,
        None => return false,
    };
    role.has_default_permission(perm)
}

/// Verifies that a role possesses all required permissions.
/// Returns false if required list is empty or role is unknown.
#[allow(dead_code)]
pub fn check_role_all_permissions(role_str: &str, required: &[Permission]) -> bool {
    if required.is_empty() {
        return false;
    }
    let role = match Role::parse(role_str) {
        Some(r) => r,
        None => return false,
    };
    required.iter().all(|p| role.has_default_permission(*p))
}

/// Verifies that a role possesses at least one of the required permissions.
/// Returns false if required list is empty or role is unknown.
#[allow(dead_code)]
pub fn check_role_any_permission(role_str: &str, required: &[Permission]) -> bool {
    if required.is_empty() {
        return false;
    }
    let role = match Role::parse(role_str) {
        Some(r) => r,
        None => return false,
    };
    required.iter().any(|p| role.has_default_permission(*p))
}

/// Validates that session tenancy/branch scope strictly matches target resource scope.
/// Role permissions never grant cross-organization or cross-branch access.
#[allow(dead_code)]
pub fn validate_scope(
    session_org_id: Option<&str>,
    session_branch_id: &str,
    target_org_id: Option<&str>,
    target_branch_id: Option<&str>,
) -> Result<(), PermissionError> {
    if let Some(target_org) = target_org_id {
        match session_org_id {
            Some(session_org) if session_org == target_org => {}
            Some(session_org) => {
                return Err(PermissionError::ScopeMismatch {
                    expected: target_org.to_string(),
                    actual: session_org.to_string(),
                });
            }
            None => {
                return Err(PermissionError::ScopeMismatch {
                    expected: target_org.to_string(),
                    actual: "none".to_string(),
                });
            }
        }
    }

    if let Some(target_branch) = target_branch_id {
        if session_branch_id != target_branch {
            return Err(PermissionError::ScopeMismatch {
                expected: target_branch.to_string(),
                actual: session_branch_id.to_string(),
            });
        }
    }

    Ok(())
}

/// Evaluates effective permission for a user considering:
/// 1. Explicit user-level override in `user_permissions` (deny takes highest precedence).
/// 2. Database `role_permissions` mapping.
/// 3. Code-defined default role permissions fallback.
/// 4. Default: DENY.
#[allow(dead_code)]
pub fn evaluate_user_permission(
    conn: &Connection,
    user_id: &str,
    role_str: &str,
    permission: Permission,
) -> Result<bool, PermissionError> {
    let perm_code = permission.as_str();

    // 1. Check user-level override
    let user_override: Option<String> = conn
        .query_row(
            "SELECT up.effect \
             FROM user_permissions up \
             JOIN permissions p ON up.permission_id = p.id \
             WHERE up.user_id = ?1 AND p.code = ?2",
            params![user_id, perm_code],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| PermissionError::Database(e.to_string()))?;

    if let Some(effect) = user_override {
        if effect.eq_ignore_ascii_case("deny") {
            return Ok(false);
        }
        if effect.eq_ignore_ascii_case("allow") {
            return Ok(true);
        }
    }

    // 2. Check database role_permissions
    let role_allowed: bool = conn
        .query_row(
            "SELECT 1 \
             FROM role_permissions rp \
             JOIN permissions p ON rp.permission_id = p.id \
             WHERE rp.role = ?1 AND p.code = ?2",
            params![role_str, perm_code],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| PermissionError::Database(e.to_string()))?
        .unwrap_or(false);

    if role_allowed {
        return Ok(true);
    }

    // 3. Fallback to code-defined default role permissions
    Ok(check_role_permission(role_str, perm_code))
}

/// Returns the complete set of effective permissions for a user.
#[allow(dead_code)]
pub fn get_effective_user_permissions(
    conn: &Connection,
    user_id: &str,
    role_str: &str,
) -> Result<HashSet<Permission>, PermissionError> {
    let mut effective = HashSet::new();

    for perm in Permission::ALL {
        if evaluate_user_permission(conn, user_id, role_str, *perm)? {
            effective.insert(*perm);
        }
    }

    Ok(effective)
}

/// Grants or denies an explicit permission override for a user in the local database.
#[allow(dead_code)]
pub fn set_user_permission_override(
    conn: &Connection,
    user_id: &str,
    permission: Permission,
    effect: &str,
) -> Result<(), PermissionError> {
    let normalized_effect = match effect.to_lowercase().as_str() {
        "allow" => "allow",
        "deny" => "deny",
        _ => {
            return Err(PermissionError::Database(format!(
                "Invalid permission effect: '{effect}'. Must be 'allow' or 'deny'"
            )))
        }
    };

    let permission_id: String = conn
        .query_row(
            "SELECT id FROM permissions WHERE code = ?1",
            params![permission.as_str()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| PermissionError::Database(e.to_string()))?
        .ok_or_else(|| PermissionError::InvalidPermission(permission.to_string()))?;

    conn.execute(
        "INSERT INTO user_permissions (user_id, permission_id, effect) \
         VALUES (?1, ?2, ?3) \
         ON CONFLICT(user_id, permission_id) DO UPDATE SET effect = excluded.effect",
        params![user_id, permission_id, normalized_effect],
    )
    .map_err(|e| PermissionError::Database(format!("Failed to set user permission: {e}")))?;

    Ok(())
}

/// Revokes an explicit user permission override, restoring base role inheritance.
#[allow(dead_code)]
pub fn remove_user_permission_override(
    conn: &Connection,
    user_id: &str,
    permission: Permission,
) -> Result<(), PermissionError> {
    let permission_id: Option<String> = conn
        .query_row(
            "SELECT id FROM permissions WHERE code = ?1",
            params![permission.as_str()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| PermissionError::Database(e.to_string()))?;

    if let Some(pid) = permission_id {
        conn.execute(
            "DELETE FROM user_permissions WHERE user_id = ?1 AND permission_id = ?2",
            params![user_id, pid],
        )
        .map_err(|e| PermissionError::Database(format!("Failed to remove user permission: {e}")))?;
    }

    Ok(())
}

/// Lists all user-level permission overrides for a given user.
#[allow(dead_code)]
pub fn list_user_permission_overrides(
    conn: &Connection,
    user_id: &str,
) -> Result<Vec<(Permission, String)>, PermissionError> {
    let mut stmt = conn
        .prepare(
            "SELECT p.code, up.effect \
             FROM user_permissions up \
             JOIN permissions p ON up.permission_id = p.id \
             WHERE up.user_id = ?1 \
             ORDER BY p.code ASC",
        )
        .map_err(|e| PermissionError::Database(e.to_string()))?;

    let rows = stmt
        .query_map(params![user_id], |row| {
            let code: String = row.get(0)?;
            let effect: String = row.get(1)?;
            Ok((code, effect))
        })
        .map_err(|e| PermissionError::Database(e.to_string()))?;

    let mut list = Vec::new();
    for row in rows {
        let (code, effect) = row.map_err(|e| PermissionError::Database(e.to_string()))?;
        if let Some(perm) = Permission::parse(&code) {
            list.push((perm, effect));
        }
    }
    Ok(list)
}
