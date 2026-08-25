// Authoritative Rust authorization middleware and boundary.
// F1.07 — Rust authorization middleware

use crate::permission::{
    evaluate_user_permission, validate_scope, Permission, PermissionError, Role,
};
use crate::user::session::{validate_local_session, SessionContext, SessionValidationError};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

/// Typed authorization errors returned by the middleware boundary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthMiddlewareError {
    Unauthenticated(String),
    SessionExpired(String),
    SessionRevoked(String),
    PermissionDenied {
        role: String,
        permission: String,
        reason: String,
    },
    ScopeMismatch {
        expected: String,
        actual: String,
    },
    Database(String),
    Validation(String),
}

impl std::fmt::Display for AuthMiddlewareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthMiddlewareError::Unauthenticated(msg) => {
                write!(f, "Authentication required: {msg}")
            }
            AuthMiddlewareError::SessionExpired(msg) => write!(f, "Session expired: {msg}"),
            AuthMiddlewareError::SessionRevoked(msg) => write!(f, "Session revoked: {msg}"),
            AuthMiddlewareError::PermissionDenied {
                role,
                permission,
                reason,
            } => {
                write!(
                    f,
                    "Permission denied: role '{role}' lacks permission '{permission}'. Reason: {reason}"
                )
            }
            AuthMiddlewareError::ScopeMismatch { expected, actual } => {
                write!(
                    f,
                    "Scope mismatch: operation requires scope '{expected}', but session has scope '{actual}'"
                )
            }
            AuthMiddlewareError::Database(msg) => write!(f, "Database error: {msg}"),
            AuthMiddlewareError::Validation(msg) => write!(f, "Validation error: {msg}"),
        }
    }
}

impl std::error::Error for AuthMiddlewareError {}

impl From<PermissionError> for AuthMiddlewareError {
    fn from(err: PermissionError) -> Self {
        match err {
            PermissionError::PermissionDenied {
                role,
                permission,
                reason,
            } => AuthMiddlewareError::PermissionDenied {
                role,
                permission,
                reason,
            },
            PermissionError::ScopeMismatch { expected, actual } => {
                AuthMiddlewareError::ScopeMismatch { expected, actual }
            }
            PermissionError::Database(msg) => AuthMiddlewareError::Database(msg),
            PermissionError::InvalidPermission(p) => {
                AuthMiddlewareError::Validation(format!("Invalid permission: {p}"))
            }
            PermissionError::InvalidRole(r) => {
                AuthMiddlewareError::Validation(format!("Invalid role: {r}"))
            }
        }
    }
}

fn map_session_error(err: SessionValidationError) -> AuthMiddlewareError {
    match err {
        SessionValidationError::Revoked => {
            AuthMiddlewareError::SessionRevoked("Session has been revoked".into())
        }
        SessionValidationError::Expired => {
            AuthMiddlewareError::SessionExpired("Session has expired".into())
        }
        SessionValidationError::NotFound => {
            AuthMiddlewareError::Unauthenticated("Session not found".into())
        }
        SessionValidationError::InactiveUser => {
            AuthMiddlewareError::Unauthenticated("User account is inactive".into())
        }
        SessionValidationError::InactiveBranch => {
            AuthMiddlewareError::Unauthenticated("Branch is inactive".into())
        }
        SessionValidationError::Database(msg) => AuthMiddlewareError::Database(msg),
    }
}

/// Declarative authorization request specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizeRequest<'a> {
    pub session_id: &'a str,
    pub permission: Option<Permission>,
    pub require_all: Option<&'a [Permission]>,
    pub require_any: Option<&'a [Permission]>,
    pub target_org_id: Option<&'a str>,
    pub target_branch_id: Option<&'a str>,
}

impl<'a> AuthorizeRequest<'a> {
    /// Creates a new base authorization request for a given session ID.
    pub fn new(session_id: &'a str) -> Self {
        Self {
            session_id,
            permission: None,
            require_all: None,
            require_any: None,
            target_org_id: None,
            target_branch_id: None,
        }
    }

    /// Specifies a single required permission.
    pub fn with_permission(mut self, permission: Permission) -> Self {
        self.permission = Some(permission);
        self
    }

    /// Specifies multiple permissions that must ALL be granted.
    #[allow(dead_code)]
    pub fn with_all_permissions(mut self, permissions: &'a [Permission]) -> Self {
        self.require_all = Some(permissions);
        self
    }

    /// Specifies multiple permissions where AT LEAST ONE must be granted.
    #[allow(dead_code)]
    pub fn with_any_permission(mut self, permissions: &'a [Permission]) -> Self {
        self.require_any = Some(permissions);
        self
    }

    /// Constrains the operation to a specific organization scope.
    #[allow(dead_code)]
    pub fn with_organization_scope(mut self, org_id: &'a str) -> Self {
        self.target_org_id = Some(org_id);
        self
    }

    /// Constrains the operation to a specific branch scope.
    #[allow(dead_code)]
    pub fn with_branch_scope(mut self, branch_id: &'a str) -> Self {
        self.target_branch_id = Some(branch_id);
        self
    }

    /// Executes the full authorization check pipeline against SQLite database state.
    #[allow(dead_code)]
    pub fn execute(&self, conn: &Connection) -> Result<SessionContext, AuthMiddlewareError> {
        authorize(conn, self)
    }
}

/// Authoritative authorization evaluation engine.
/// Enforces:
/// 1. Active, unexpired, unrevoked local session with active user and branch.
/// 2. Organization and branch tenancy scope boundaries (even admin cannot bypass).
/// 3. Single and multi-permission checks with exact matching and explicit user deny precedence.
#[allow(dead_code)]
pub fn authorize(
    conn: &Connection,
    req: &AuthorizeRequest<'_>,
) -> Result<SessionContext, AuthMiddlewareError> {
    if req.session_id.trim().is_empty() {
        return Err(AuthMiddlewareError::Unauthenticated(
            "Session ID is required".into(),
        ));
    }

    // 1. Authenticate and validate local session
    let session = validate_local_session(conn, req.session_id).map_err(map_session_error)?;

    // 2. Validate tenant and branch scope boundaries
    validate_scope(
        session.organization_id.as_deref(),
        &session.branch_id,
        req.target_org_id,
        req.target_branch_id,
    )?;

    // 3. Enforce single permission requirement
    if let Some(perm) = req.permission {
        evaluate_single_permission(conn, &session, perm)?;
    }

    // 4. Enforce multi-permission ALL requirement (AND semantics)
    if let Some(all_perms) = req.require_all {
        if all_perms.is_empty() {
            return Err(AuthMiddlewareError::PermissionDenied {
                role: session.role.clone(),
                permission: "none".to_string(),
                reason: "Empty permission list fails closed".into(),
            });
        }
        for perm in all_perms {
            evaluate_single_permission(conn, &session, *perm)?;
        }
    }

    // 5. Enforce multi-permission ANY requirement (OR semantics)
    if let Some(any_perms) = req.require_any {
        evaluate_any_permissions(conn, &session, any_perms)?;
    }

    Ok(session)
}

fn evaluate_single_permission(
    conn: &Connection,
    session: &SessionContext,
    permission: Permission,
) -> Result<(), AuthMiddlewareError> {
    // Validate role exists
    if Role::parse(&session.role).is_none() {
        return Err(AuthMiddlewareError::PermissionDenied {
            role: session.role.clone(),
            permission: permission.as_str().to_string(),
            reason: "Unknown or invalid role fails closed".into(),
        });
    }

    let allowed = evaluate_user_permission(conn, &session.user_id, &session.role, permission)?;

    if !allowed {
        return Err(AuthMiddlewareError::PermissionDenied {
            role: session.role.clone(),
            permission: permission.as_str().to_string(),
            reason: "Role or user override does not grant this permission".into(),
        });
    }

    Ok(())
}

fn evaluate_any_permissions(
    conn: &Connection,
    session: &SessionContext,
    permissions: &[Permission],
) -> Result<(), AuthMiddlewareError> {
    if permissions.is_empty() {
        return Err(AuthMiddlewareError::PermissionDenied {
            role: session.role.clone(),
            permission: "none".to_string(),
            reason: "Empty permission list fails closed".into(),
        });
    }

    if Role::parse(&session.role).is_none() {
        return Err(AuthMiddlewareError::PermissionDenied {
            role: session.role.clone(),
            permission: "any".to_string(),
            reason: "Unknown or invalid role fails closed".into(),
        });
    }

    for perm in permissions {
        if evaluate_user_permission(conn, &session.user_id, &session.role, *perm)? {
            return Ok(());
        }
    }

    Err(AuthMiddlewareError::PermissionDenied {
        role: session.role.clone(),
        permission: "any".to_string(),
        reason: "User possesses none of the required permissions".into(),
    })
}

/// Convenience helper: requires an active session with a single permission.
#[allow(dead_code)]
pub fn require_permission(
    conn: &Connection,
    session_id: &str,
    permission: Permission,
) -> Result<SessionContext, AuthMiddlewareError> {
    AuthorizeRequest::new(session_id)
        .with_permission(permission)
        .execute(conn)
}

/// Convenience helper: requires an active session with a scoped single permission.
#[allow(dead_code)]
pub fn require_scoped_permission(
    conn: &Connection,
    session_id: &str,
    permission: Permission,
    target_org: Option<&str>,
    target_branch: Option<&str>,
) -> Result<SessionContext, AuthMiddlewareError> {
    let mut req = AuthorizeRequest::new(session_id).with_permission(permission);
    if let Some(org) = target_org {
        req = req.with_organization_scope(org);
    }
    if let Some(branch) = target_branch {
        req = req.with_branch_scope(branch);
    }
    req.execute(conn)
}

/// Convenience helper: requires an active session with no specific permission requirement.
#[allow(dead_code)]
pub fn require_session(
    conn: &Connection,
    session_id: &str,
) -> Result<SessionContext, AuthMiddlewareError> {
    AuthorizeRequest::new(session_id).execute(conn)
}

/// Convenience helper: requires all specified permissions.
#[allow(dead_code)]
pub fn require_all_permissions(
    conn: &Connection,
    session_id: &str,
    permissions: &[Permission],
) -> Result<SessionContext, AuthMiddlewareError> {
    AuthorizeRequest::new(session_id)
        .with_all_permissions(permissions)
        .execute(conn)
}

/// Convenience helper: requires at least one of the specified permissions.
#[allow(dead_code)]
pub fn require_any_permission(
    conn: &Connection,
    session_id: &str,
    permissions: &[Permission],
) -> Result<SessionContext, AuthMiddlewareError> {
    AuthorizeRequest::new(session_id)
        .with_any_permission(permissions)
        .execute(conn)
}
