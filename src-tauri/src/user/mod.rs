// Local User domain model and repository operations.
// F1.05 — Local user/session model

pub mod session;

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: String,
    pub branch_id: String,
    pub full_name: String,
    pub username: Option<String>,
    pub role: String,
    pub is_active: bool,
    pub supabase_user_id: Option<String>,
    pub auth_provider: String,
    pub created_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserInput {
    pub branch_id: String,
    pub full_name: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub pin: Option<String>,
    pub role: String,
    pub supabase_user_id: Option<String>,
    pub auth_provider: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserInput {
    pub full_name: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub pin: Option<String>,
    pub role: Option<String>,
    pub is_active: Option<bool>,
    pub supabase_user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UserError {
    Validation(String),
    NotFound(String),
    BranchNotFound(String),
    DuplicateUsername(String),
    DuplicateSupabaseId(String),
    InvalidCredentials(String),
    Database(String),
}

impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserError::Validation(msg) => write!(f, "Validation error: {msg}"),
            UserError::NotFound(msg) => write!(f, "User not found: {msg}"),
            UserError::BranchNotFound(msg) => write!(f, "Branch not found: {msg}"),
            UserError::DuplicateUsername(msg) => write!(f, "Username already exists: {msg}"),
            UserError::DuplicateSupabaseId(msg) => {
                write!(f, "Supabase user ID already mapped: {msg}")
            }
            UserError::InvalidCredentials(msg) => write!(f, "Invalid credentials: {msg}"),
            UserError::Database(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

impl std::error::Error for UserError {}

/// Precomputed valid Argon2id hash for constant-time decoy verification to defeat timing-based user enumeration.
pub const DECOY_ARGON2_HASH: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$ZGVjb3lzYWx0MTIzNDU2Nw$DecoyHashToPreventUserEnumerationTimingAttack123";

/// Generates a cryptographically secure Argon2id hash in standard PHC format.
#[allow(dead_code)]
pub fn hash_secret(secret: &str) -> Result<String, UserError> {
    let salt_bytes = uuid::Uuid::new_v4();
    let salt = SaltString::encode_b64(salt_bytes.as_bytes())
        .map_err(|e| UserError::Database(format!("Salt encoding error: {e}")))?;
    let argon2 = Argon2::default();
    argon2
        .hash_password(secret.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| UserError::Database(format!("Argon2 hashing error: {e}")))
}

/// Verifies a candidate secret against a stored Argon2id PHC string.
pub fn verify_secret(candidate: &str, stored_hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(stored_hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(candidate.as_bytes(), &parsed_hash)
        .is_ok()
}

struct RateLimiterState {
    map: HashMap<String, (u32, Instant)>,
    client_saturated_until: HashMap<String, Instant>,
    client_overflow_failures: HashMap<String, u32>,
}

impl RateLimiterState {
    fn update_existing(&mut self, rate_key: &str, max_attempts: u32, lockout: Duration) -> bool {
        if let Some(entry) = self.map.get_mut(rate_key) {
            if entry.0 >= max_attempts && entry.1.elapsed() >= lockout {
                *entry = (1, Instant::now());
            } else {
                entry.0 += 1;
                entry.1 = Instant::now();
            }
            true
        } else {
            false
        }
    }

    fn purge_expired(&mut self, lockout: Duration) {
        self.map
            .retain(|_, (_, last_failed)| last_failed.elapsed() < lockout);
        self.client_saturated_until
            .retain(|_, until| Instant::now() < *until);
    }

    fn evict_oldest_non_locked(&mut self, max_attempts: u32) {
        let oldest_non_locked = self
            .map
            .iter()
            .filter(|(_, (count, _))| *count < max_attempts)
            .min_by_key(|(_, (_, time))| *time)
            .map(|(k, _)| k.clone());

        if let Some(non_locked_key) = oldest_non_locked {
            self.map.remove(&non_locked_key);
        }
    }

    fn insert_or_throttle(
        &mut self,
        rate_key: &str,
        client_id: &str,
        max_attempts: u32,
        max_entries: usize,
        lockout: Duration,
    ) {
        if self.map.len() < max_entries {
            self.map.insert(rate_key.to_string(), (1, Instant::now()));
        } else {
            let count = self
                .client_overflow_failures
                .entry(client_id.to_string())
                .or_insert(0);
            *count += 1;
            if *count >= max_attempts {
                self.client_saturated_until
                    .insert(client_id.to_string(), Instant::now() + lockout);
            }
        }
    }
}

/// Thread-safe in-memory rate limiter with bounded memory growth, strict active-lockout preservation,
/// client-scoped admission throttling under saturation, and composite client scoping.
pub struct RateLimiter {
    state: Mutex<RateLimiterState>,
    max_attempts: u32,
    lockout_duration: Duration,
    max_entries: usize,
}

impl RateLimiter {
    pub fn new(max_attempts: u32, lockout_secs: u64, max_entries: usize) -> Self {
        Self {
            state: Mutex::new(RateLimiterState {
                map: HashMap::new(),
                client_saturated_until: HashMap::new(),
                client_overflow_failures: HashMap::new(),
            }),
            max_attempts,
            lockout_duration: Duration::from_secs(lockout_secs),
            max_entries,
        }
    }

    /// Returns the number of currently tracked entries (useful for test assertions).
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.state.lock().map(|s| s.map.len()).unwrap_or(0)
    }

    /// Checks if the limiter has no tracked entries.
    #[cfg(test)]
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.state.lock().map(|s| s.map.is_empty()).unwrap_or(true)
    }

    /// Checks if a client-identity key is currently locked out or subject to client admission throttle.
    pub fn check(&self, rate_key: &str) -> Result<(), UserError> {
        let mut state = self
            .state
            .lock()
            .map_err(|e| UserError::Database(e.to_string()))?;

        let client_id = rate_key.split(':').next().unwrap_or(rate_key);

        if let Some((count, last_failed)) = state.map.get(rate_key) {
            if *count >= self.max_attempts {
                if last_failed.elapsed() < self.lockout_duration {
                    return Err(UserError::InvalidCredentials(
                        "Too many failed attempts. Account is temporarily locked. Please try again later.".into(),
                    ));
                }
                state.map.remove(rate_key);
            }
        } else if let Some(until) = state.client_saturated_until.get(client_id) {
            if Instant::now() < *until {
                return Err(UserError::InvalidCredentials(
                    "Too many failed attempts. Authentication service is temporarily throttling requests from this client. Please try again later.".into(),
                ));
            }
            state.client_saturated_until.remove(client_id);
            state.client_overflow_failures.remove(client_id);
        }
        Ok(())
    }

    /// Records a failure. Active lockouts are strictly preserved and never evicted.
    /// Under total saturation of active lockouts, client-scoped admission throttling prevents unthrottled bypass.
    pub fn record_failure(&self, rate_key: &str) {
        if let Ok(mut state) = self.state.lock() {
            if state.update_existing(rate_key, self.max_attempts, self.lockout_duration) {
                return;
            }

            let client_id = rate_key.split(':').next().unwrap_or(rate_key);
            state.purge_expired(self.lockout_duration);

            if state.map.len() >= self.max_entries {
                state.evict_oldest_non_locked(self.max_attempts);
            }

            state.insert_or_throttle(
                rate_key,
                client_id,
                self.max_attempts,
                self.max_entries,
                self.lockout_duration,
            );
        }
    }

    /// Clears the failure record on successful authentication.
    pub fn record_success(&self, rate_key: &str) {
        if let Ok(mut state) = self.state.lock() {
            state.map.remove(rate_key);
            let client_id = rate_key.split(':').next().unwrap_or(rate_key);
            state.client_overflow_failures.remove(client_id);
        }
    }

    /// Clears all entries (useful in testing).
    #[cfg(test)]
    #[allow(dead_code)]
    pub fn reset_all(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.map.clear();
            state.client_saturated_until.clear();
            state.client_overflow_failures.clear();
        }
    }
}

/// Global rate limiter enforcing max 5 failed attempts per client identity with 30-second lockout.
static GLOBAL_AUTH_RATE_LIMITER: OnceLock<RateLimiter> = OnceLock::new();

/// Exposes the global rate limiter for production operations.
pub fn get_auth_rate_limiter() -> &'static RateLimiter {
    GLOBAL_AUTH_RATE_LIMITER.get_or_init(|| RateLimiter::new(5, 30, 10_000))
}

/// Maps a database row to a User entity.
pub fn map_user_row(row: &Row<'_>) -> rusqlite::Result<User> {
    Ok(User {
        id: row.get(0)?,
        branch_id: row.get(1)?,
        full_name: row.get(2)?,
        username: row.get(3)?,
        role: row.get(4)?,
        is_active: row.get::<_, i64>(5)? == 1,
        supabase_user_id: row.get(6)?,
        auth_provider: row.get(7)?,
        created_at: row.get(8)?,
    })
}

fn validate_create_user_input(input: &CreateUserInput) -> Result<(), UserError> {
    let full_name = input.full_name.trim();
    if full_name.is_empty() {
        return Err(UserError::Validation(
            "User full name cannot be empty".into(),
        ));
    }
    if full_name.chars().count() > 255 {
        return Err(UserError::Validation(
            "User full name cannot exceed 255 characters".into(),
        ));
    }
    if input.role.trim().is_empty() {
        return Err(UserError::Validation("User role cannot be empty".into()));
    }
    if input.branch_id.trim().is_empty() {
        return Err(UserError::Validation("Branch ID cannot be empty".into()));
    }
    Ok(())
}

fn verify_branch_exists(conn: &Connection, branch_id: &str) -> Result<(), UserError> {
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM branches WHERE id = ?1",
            params![branch_id],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| UserError::Database(e.to_string()))?
        .unwrap_or(false);

    if !exists {
        Err(UserError::BranchNotFound(format!(
            "Branch '{branch_id}' does not exist"
        )))
    } else {
        Ok(())
    }
}

fn validate_new_username(
    conn: &Connection,
    username: Option<&str>,
) -> Result<Option<String>, UserError> {
    let raw = match username {
        Some(u) if !u.trim().is_empty() => u.trim(),
        _ => return Ok(None),
    };

    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM users WHERE username = ?1",
            params![raw],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| UserError::Database(e.to_string()))?
        .unwrap_or(false);

    if exists {
        Err(UserError::DuplicateUsername(format!(
            "Username '{raw}' already exists"
        )))
    } else {
        Ok(Some(raw.to_string()))
    }
}

fn validate_new_supabase_id(
    conn: &Connection,
    supabase_id: Option<&str>,
) -> Result<Option<String>, UserError> {
    let raw = match supabase_id {
        Some(s) if !s.trim().is_empty() => s.trim(),
        _ => return Ok(None),
    };

    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM users WHERE supabase_user_id = ?1",
            params![raw],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| UserError::Database(e.to_string()))?
        .unwrap_or(false);

    if exists {
        Err(UserError::DuplicateSupabaseId(format!(
            "Supabase user ID '{raw}' is already linked to a user"
        )))
    } else {
        Ok(Some(raw.to_string()))
    }
}

fn hash_credential_field(secret: Option<&str>) -> Result<Option<String>, UserError> {
    match secret {
        Some(s) if !s.trim().is_empty() => Ok(Some(hash_secret(s)?)),
        _ => Ok(None),
    }
}

/// Creates a new user record in the local database.
#[allow(dead_code)]
pub fn create_user(conn: &Connection, input: CreateUserInput) -> Result<User, UserError> {
    validate_create_user_input(&input)?;
    let branch_id = input.branch_id.trim();
    verify_branch_exists(conn, branch_id)?;

    let username = validate_new_username(conn, input.username.as_deref())?;
    let supabase_user_id = validate_new_supabase_id(conn, input.supabase_user_id.as_deref())?;
    let password_hash = hash_credential_field(input.password.as_deref())?;
    let pin_hash = hash_credential_field(input.pin.as_deref())?;
    let auth_provider = input.auth_provider.unwrap_or_else(|| "local".into());
    let user_id = uuid::Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO users (id, branch_id, full_name, username, password_hash, pin_hash, role, is_active, supabase_user_id, auth_provider, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, ?9, datetime('now'))",
        params![
            user_id,
            branch_id,
            input.full_name.trim(),
            username,
            password_hash,
            pin_hash,
            input.role.trim(),
            supabase_user_id,
            auth_provider,
        ],
    )
    .map_err(|e| UserError::Database(format!("Failed to create user: {e}")))?;

    get_user(conn, &user_id)
}

/// Retrieves a user by ID.
#[allow(dead_code)]
pub fn get_user(conn: &Connection, id: &str) -> Result<User, UserError> {
    conn.query_row(
        "SELECT id, branch_id, full_name, username, role, is_active, supabase_user_id, auth_provider, created_at
         FROM users WHERE id = ?1",
        params![id],
        map_user_row,
    )
    .optional()
    .map_err(|e| UserError::Database(e.to_string()))?
    .ok_or_else(|| UserError::NotFound(format!("User '{id}' not found")))
}

/// Retrieves a user by username.
#[allow(dead_code)]
pub fn get_user_by_username(conn: &Connection, username: &str) -> Result<User, UserError> {
    conn.query_row(
        "SELECT id, branch_id, full_name, username, role, is_active, supabase_user_id, auth_provider, created_at
         FROM users WHERE username = ?1",
        params![username.trim()],
        map_user_row,
    )
    .optional()
    .map_err(|e| UserError::Database(e.to_string()))?
    .ok_or_else(|| UserError::NotFound(format!("User with username '{username}' not found")))
}

/// Retrieves a user by linked Supabase user ID.
#[allow(dead_code)]
pub fn get_user_by_supabase_id(
    conn: &Connection,
    supabase_user_id: &str,
) -> Result<User, UserError> {
    conn.query_row(
        "SELECT id, branch_id, full_name, username, role, is_active, supabase_user_id, auth_provider, created_at
         FROM users WHERE supabase_user_id = ?1",
        params![supabase_user_id.trim()],
        map_user_row,
    )
    .optional()
    .map_err(|e| UserError::Database(e.to_string()))?
    .ok_or_else(|| UserError::NotFound(format!("User with Supabase ID '{supabase_user_id}' not found")))
}

fn validate_update_name_and_role(input: &UpdateUserInput) -> Result<(), UserError> {
    if let Some(ref name) = input.full_name {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(UserError::Validation("Full name cannot be empty".into()));
        }
        if trimmed.chars().count() > 255 {
            return Err(UserError::Validation(
                "Full name cannot exceed 255 characters".into(),
            ));
        }
    }
    if let Some(ref role) = input.role {
        if role.trim().is_empty() {
            return Err(UserError::Validation("Role cannot be empty".into()));
        }
    }
    Ok(())
}

fn validate_update_username(
    conn: &Connection,
    user_id: &str,
    current_username: Option<&str>,
    new_username: Option<&str>,
) -> Result<Option<Option<String>>, UserError> {
    let raw = match new_username {
        Some(u) => u.trim(),
        None => return Ok(None),
    };

    if raw.is_empty() {
        return Ok(Some(None));
    }

    if Some(raw) == current_username {
        return Ok(Some(Some(raw.to_string())));
    }

    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM users WHERE username = ?1 AND id != ?2",
            params![raw, user_id],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| UserError::Database(e.to_string()))?
        .unwrap_or(false);

    if exists {
        Err(UserError::DuplicateUsername(format!(
            "Username '{raw}' already exists"
        )))
    } else {
        Ok(Some(Some(raw.to_string())))
    }
}

fn validate_update_supabase_id(
    conn: &Connection,
    user_id: &str,
    current_supabase_id: Option<&str>,
    new_supabase_id: Option<&str>,
) -> Result<Option<Option<String>>, UserError> {
    let raw = match new_supabase_id {
        Some(s) => s.trim(),
        None => return Ok(None),
    };

    if raw.is_empty() {
        return Ok(Some(None));
    }

    if Some(raw) == current_supabase_id {
        return Ok(Some(Some(raw.to_string())));
    }

    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM users WHERE supabase_user_id = ?1 AND id != ?2",
            params![raw, user_id],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| UserError::Database(e.to_string()))?
        .unwrap_or(false);

    if exists {
        Err(UserError::DuplicateSupabaseId(format!(
            "Supabase ID '{raw}' already mapped"
        )))
    } else {
        Ok(Some(Some(raw.to_string())))
    }
}

struct UserUpdateBuilder {
    clauses: Vec<&'static str>,
    params: Vec<Box<dyn rusqlite::ToSql>>,
}

impl UserUpdateBuilder {
    fn new() -> Self {
        Self {
            clauses: Vec::new(),
            params: Vec::new(),
        }
    }

    fn push_text_field(&mut self, clause: &'static str, val: Option<&str>) {
        if let Some(s) = val {
            self.clauses.push(clause);
            self.params.push(Box::new(s.trim().to_string()));
        }
    }

    fn push_nullable_field(&mut self, clause: &'static str, val: Option<Option<String>>) {
        if let Some(opt) = val {
            self.clauses.push(clause);
            self.params.push(Box::new(opt));
        }
    }

    fn push_hash_field(
        &mut self,
        clause: &'static str,
        raw: Option<&str>,
    ) -> Result<(), UserError> {
        if let Some(secret) = raw {
            let hash = if secret.trim().is_empty() {
                None
            } else {
                Some(hash_secret(secret)?)
            };
            self.clauses.push(clause);
            self.params.push(Box::new(hash));
        }
        Ok(())
    }

    fn push_bool_field(&mut self, clause: &'static str, val: Option<bool>) {
        if let Some(b) = val {
            self.clauses.push(clause);
            self.params.push(Box::new(if b { 1i64 } else { 0i64 }));
        }
    }

    fn execute(self, conn: &Connection, user_id: &str) -> Result<bool, UserError> {
        if self.clauses.is_empty() {
            return Ok(false);
        }
        let query = format!("UPDATE users SET {} WHERE id = ?", self.clauses.join(", "));
        let mut params = self.params;
        params.push(Box::new(user_id.to_string()));
        let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(Box::as_ref).collect();
        conn.execute(&query, params_ref.as_slice())
            .map_err(|e| UserError::Database(format!("Failed to update user: {e}")))?;
        Ok(true)
    }
}

/// Updates user fields safely.
#[allow(dead_code)]
pub fn update_user(conn: &Connection, id: &str, input: UpdateUserInput) -> Result<User, UserError> {
    let existing = get_user(conn, id)?;
    validate_update_name_and_role(&input)?;

    let validated_username = validate_update_username(
        conn,
        id,
        existing.username.as_deref(),
        input.username.as_deref(),
    )?;
    let validated_supabase_id = validate_update_supabase_id(
        conn,
        id,
        existing.supabase_user_id.as_deref(),
        input.supabase_user_id.as_deref(),
    )?;

    let mut builder = UserUpdateBuilder::new();
    builder.push_text_field("full_name = ?", input.full_name.as_deref());
    builder.push_nullable_field("username = ?", validated_username);
    builder.push_hash_field("password_hash = ?", input.password.as_deref())?;
    builder.push_hash_field("pin_hash = ?", input.pin.as_deref())?;
    builder.push_text_field("role = ?", input.role.as_deref());
    builder.push_bool_field("is_active = ?", input.is_active);
    builder.push_nullable_field("supabase_user_id = ?", validated_supabase_id);

    let updated = builder.execute(conn, id)?;
    if updated {
        get_user(conn, id)
    } else {
        Ok(existing)
    }
}

/// Lists all users for a given branch.
#[allow(dead_code)]
pub fn list_users(conn: &Connection, branch_id: &str) -> Result<Vec<User>, UserError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, branch_id, full_name, username, role, is_active, supabase_user_id, auth_provider, created_at
             FROM users WHERE branch_id = ?1
             ORDER BY created_at ASC, id ASC",
        )
        .map_err(|e| UserError::Database(e.to_string()))?;

    let rows = stmt
        .query_map(params![branch_id], map_user_row)
        .map_err(|e| UserError::Database(e.to_string()))?;

    let mut users = Vec::new();
    for row in rows {
        users.push(row.map_err(|e| UserError::Database(e.to_string()))?);
    }
    Ok(users)
}

/// Shared internal credential verification with timing decoy defense and rate limiting.
fn verify_credential_internal(
    limiter: &RateLimiter,
    rate_key: &str,
    user_and_hash: Option<(User, Option<String>)>,
    candidate: &str,
    error_msg: &'static str,
) -> Result<User, UserError> {
    match user_and_hash {
        Some((user, Some(stored_hash))) => {
            if !verify_secret(candidate, &stored_hash) {
                limiter.record_failure(rate_key);
                return Err(UserError::InvalidCredentials(error_msg.into()));
            }
            if !user.is_active {
                limiter.record_failure(rate_key);
                return Err(UserError::InvalidCredentials(error_msg.into()));
            }
            limiter.record_success(rate_key);
            Ok(user)
        }
        _ => {
            let _ = verify_secret(candidate, DECOY_ARGON2_HASH);
            limiter.record_failure(rate_key);
            Err(UserError::InvalidCredentials(error_msg.into()))
        }
    }
}

/// Verifies user password with injectable rate limiter and client scoping.
pub fn verify_user_password_with_limiter(
    conn: &Connection,
    limiter: &RateLimiter,
    client_id: &str,
    username: &str,
    password: &str,
) -> Result<User, UserError> {
    let rate_key = format!("{client_id}:user:{}", username.trim().to_lowercase());
    limiter.check(&rate_key)?;

    let user_opt: Option<(User, Option<String>)> = conn
        .query_row(
            "SELECT id, branch_id, full_name, username, role, is_active, supabase_user_id, auth_provider, created_at, password_hash
             FROM users WHERE username = ?1",
            params![username.trim()],
            |row| Ok((map_user_row(row)?, row.get::<_, Option<String>>(9)?)),
        )
        .optional()
        .map_err(|e| UserError::Database(e.to_string()))?;

    verify_credential_internal(
        limiter,
        &rate_key,
        user_opt,
        password,
        "Invalid username or password",
    )
}

/// Verifies user password using the global rate limiter and standard client context.
pub fn verify_user_password(
    conn: &Connection,
    username: &str,
    password: &str,
) -> Result<User, UserError> {
    verify_user_password_with_limiter(
        conn,
        get_auth_rate_limiter(),
        "local_terminal",
        username,
        password,
    )
}

/// Verifies user cashier PIN with injectable rate limiter and client scoping.
pub fn verify_user_pin_with_limiter(
    conn: &Connection,
    limiter: &RateLimiter,
    client_id: &str,
    user_id: &str,
    pin: &str,
) -> Result<User, UserError> {
    let rate_key = format!("{client_id}:pin:{}", user_id.trim());
    limiter.check(&rate_key)?;

    let user_opt: Option<(User, Option<String>)> = conn
        .query_row(
            "SELECT id, branch_id, full_name, username, role, is_active, supabase_user_id, auth_provider, created_at, pin_hash
             FROM users WHERE id = ?1",
            params![user_id.trim()],
            |row| Ok((map_user_row(row)?, row.get::<_, Option<String>>(9)?)),
        )
        .optional()
        .map_err(|e| UserError::Database(e.to_string()))?;

    verify_credential_internal(limiter, &rate_key, user_opt, pin, "Invalid PIN")
}

/// Verifies user cashier PIN using the global rate limiter and standard client context.
pub fn verify_user_pin(conn: &Connection, user_id: &str, pin: &str) -> Result<User, UserError> {
    verify_user_pin_with_limiter(
        conn,
        get_auth_rate_limiter(),
        "local_terminal",
        user_id,
        pin,
    )
}
