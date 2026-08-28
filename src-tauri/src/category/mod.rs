// Category domain model, validation rules, hierarchy invariants, and database operations.
// F2.02 — Categories, Brands, Manufacturers

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Canonical Category entity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Recursive category tree node for hierarchical UI and navigation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CategoryTreeNode {
    pub category: Category,
    pub children: Vec<CategoryTreeNode>,
}

/// Input payload for creating a category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCategoryInput {
    pub name: String,
    pub parent_id: Option<String>,
    pub description: Option<String>,
}

/// Input payload for updating a category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCategoryInput {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub description: Option<String>,
    pub is_active: bool,
}

/// Filter parameters for listing categories.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CategoryFilter {
    pub query: Option<String>,
    /// None = all; Some(None) = roots only (parent_id IS NULL); Some(Some(id)) = children of id
    pub parent_id: Option<Option<String>>,
    pub is_active: Option<bool>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CategoryError {
    Validation(String),
    NotFound(String),
    DuplicateName(String),
    InactiveParent(String),
    SelfParenting(String),
    CycleDetected(String),
    HierarchyDepthExceeded(String),
    HasActiveChildren(String),
    Database(String),
}

impl std::fmt::Display for CategoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CategoryError::Validation(msg) => write!(f, "Validation error: {msg}"),
            CategoryError::NotFound(msg) => write!(f, "Not found: {msg}"),
            CategoryError::DuplicateName(msg) => write!(f, "Duplicate name error: {msg}"),
            CategoryError::InactiveParent(msg) => write!(f, "Inactive parent error: {msg}"),
            CategoryError::SelfParenting(msg) => write!(f, "Self parenting error: {msg}"),
            CategoryError::CycleDetected(msg) => write!(f, "Cycle detected error: {msg}"),
            CategoryError::HierarchyDepthExceeded(msg) => {
                write!(f, "Hierarchy depth exceeded: {msg}")
            }
            CategoryError::HasActiveChildren(msg) => {
                write!(f, "Category has active children: {msg}")
            }
            CategoryError::Database(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

impl std::error::Error for CategoryError {}

impl From<rusqlite::Error> for CategoryError {
    fn from(e: rusqlite::Error) -> Self {
        if let rusqlite::Error::SqliteFailure(ref f, Some(ref msg)) = e {
            if f.code == rusqlite::ffi::ErrorCode::ConstraintViolation {
                if msg.contains("idx_categories_root_name_active")
                    || msg.contains("idx_categories_sibling_name_active")
                    || msg.contains("UNIQUE constraint failed: categories.name")
                {
                    return CategoryError::DuplicateName(
                        "An active category with this name already exists in this scope".into(),
                    );
                }
                if msg.contains("FOREIGN KEY constraint failed") {
                    return CategoryError::NotFound("Parent category not found".into());
                }
            }
        }
        CategoryError::Database(e.to_string())
    }
}

/// Escapes wildcard characters (% and _) in user queries for SQL LIKE with ESCAPE '\\'.
pub fn escape_like_pattern(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for c in input.chars() {
        if c == '%' || c == '_' || c == '\\' {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

/// Validates category name. Must be non-empty and <= 255 Unicode characters.
pub fn validate_name(name: &str) -> Result<String, CategoryError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(CategoryError::Validation(
            "Category name cannot be empty".into(),
        ));
    }
    if trimmed.chars().count() > 255 {
        return Err(CategoryError::Validation(
            "Category name exceeds maximum length of 255 characters".into(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Validates category description. Trims whitespace and normalizes empty string to None.
pub fn validate_description(desc: Option<&str>) -> Option<String> {
    desc.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

/// Normalizes parent_id. Trims whitespace and converts empty string to None.
pub fn normalize_parent_id(pid: Option<&str>) -> Option<String> {
    pid.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

const CATEGORY_COLUMNS: &str =
    "id, name, parent_id, description, is_active, created_at, updated_at";

fn map_category_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Category> {
    let is_active_int: i64 = row.get("is_active")?;
    Ok(Category {
        id: row.get("id")?,
        name: row.get("name")?,
        parent_id: row.get("parent_id")?,
        description: row.get("description")?,
        is_active: is_active_int != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// Checks that a target parent category exists and is active.
fn ensure_parent_active(conn: &Connection, parent_id: &str) -> Result<(), CategoryError> {
    let parent_active: Option<i64> = conn
        .query_row(
            "SELECT is_active FROM categories WHERE id = ?1",
            [parent_id],
            |row| row.get(0),
        )
        .optional()?;

    match parent_active {
        None => Err(CategoryError::NotFound(format!(
            "Parent category with ID '{parent_id}' not found"
        ))),
        Some(0) => Err(CategoryError::InactiveParent(format!(
            "Parent category '{parent_id}' is inactive/archived"
        ))),
        Some(_) => Ok(()),
    }
}

/// Defensive cycle detection: Walks ancestor chain from `target_parent_id` upwards.
/// Returns `Err(CategoryError::CycleDetected)` if `target_parent_id` or any ancestor equals `category_id`.
pub fn check_category_cycle(
    conn: &Connection,
    category_id: &str,
    target_parent_id: &str,
) -> Result<(), CategoryError> {
    if category_id == target_parent_id {
        return Err(CategoryError::SelfParenting(
            "Category cannot be its own parent".into(),
        ));
    }

    let mut current = target_parent_id.to_string();
    let mut steps = 0;
    const MAX_DEFENSIVE_STEPS: usize = 50;

    while steps < MAX_DEFENSIVE_STEPS {
        let parent_of_current: Option<Option<String>> = conn
            .query_row(
                "SELECT parent_id FROM categories WHERE id = ?1",
                [&current],
                |row| row.get(0),
            )
            .optional()?;

        match parent_of_current {
            None => {
                // Ancestor doesn't exist; referential integrity will be caught
                break;
            }
            Some(None) => {
                // Reached a root category; no cycle
                return Ok(());
            }
            Some(Some(ancestor_id)) => {
                if ancestor_id == category_id {
                    return Err(CategoryError::CycleDetected(format!(
                        "Category '{category_id}' cannot be parented under its own descendant '{target_parent_id}'"
                    )));
                }
                current = ancestor_id;
                steps += 1;
            }
        }
    }

    if steps >= MAX_DEFENSIVE_STEPS {
        return Err(CategoryError::HierarchyDepthExceeded(format!(
            "Hierarchy depth exceeds supported maximum of {MAX_DEFENSIVE_STEPS} levels"
        )));
    }

    Ok(())
}

/// Creates a new category.
pub fn create_category(
    conn: &Connection,
    input: CreateCategoryInput,
) -> Result<Category, CategoryError> {
    let name = validate_name(&input.name)?;
    let description = validate_description(input.description.as_deref());
    let parent_id = normalize_parent_id(input.parent_id.as_deref());

    if let Some(ref pid) = parent_id {
        ensure_parent_active(conn, pid)?;
    }

    let id = uuid::Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO categories (
            id, name, parent_id, description, is_active, created_at, updated_at
        ) VALUES (
            ?1, ?2, ?3, ?4, 1, datetime('now'), datetime('now')
        )",
        params![id, name, parent_id, description],
    )?;

    get_category(conn, &id)?
        .ok_or_else(|| CategoryError::Database("Failed to retrieve created category".into()))
}

/// Retrieves a category by unique ID.
pub fn get_category(conn: &Connection, id: &str) -> Result<Option<Category>, CategoryError> {
    let sql = format!("SELECT {CATEGORY_COLUMNS} FROM categories WHERE id = ?1");
    let result = conn.query_row(&sql, [id], map_category_row).optional()?;
    Ok(result)
}

/// Updates an existing category.
pub fn update_category(
    conn: &Connection,
    input: UpdateCategoryInput,
) -> Result<Category, CategoryError> {
    let name = validate_name(&input.name)?;
    let description = validate_description(input.description.as_deref());
    let parent_id = normalize_parent_id(input.parent_id.as_deref());

    let existing = get_category(conn, &input.id)?
        .ok_or_else(|| CategoryError::NotFound(format!("Category '{}' not found", input.id)))?;

    // Parent / cycle validation if parent_id is set
    if let Some(ref pid) = parent_id {
        if pid == &input.id {
            return Err(CategoryError::SelfParenting(
                "Category cannot be its own parent".into(),
            ));
        }
        ensure_parent_active(conn, pid)?;
        check_category_cycle(conn, &input.id, pid)?;
    }

    // Archive guard: If deactivating an active category, verify no active subcategories exist
    if existing.is_active && !input.is_active {
        let active_children: i64 = conn.query_row(
            "SELECT COUNT(*) FROM categories WHERE parent_id = ?1 AND is_active = 1",
            [&input.id],
            |row| row.get(0),
        )?;
        if active_children > 0 {
            return Err(CategoryError::HasActiveChildren(format!(
                "Cannot archive category '{}' because it has {} active subcategory(ies)",
                input.id, active_children
            )));
        }
    }

    let is_active_int = if input.is_active { 1 } else { 0 };

    let affected = conn.execute(
        "UPDATE categories SET
            name = ?1,
            parent_id = ?2,
            description = ?3,
            is_active = ?4,
            updated_at = datetime('now')
        WHERE id = ?5",
        params![name, parent_id, description, is_active_int, input.id],
    )?;

    if affected == 0 {
        return Err(CategoryError::NotFound(format!(
            "Category '{}' not found",
            input.id
        )));
    }

    get_category(conn, &input.id)?
        .ok_or_else(|| CategoryError::Database("Failed to retrieve updated category".into()))
}

/// Soft-deletes a category by setting `is_active = 0`.
/// Rejects deletion if active subcategories exist.
pub fn delete_category(conn: &Connection, id: &str) -> Result<(), CategoryError> {
    let existing = get_category(conn, id)?
        .ok_or_else(|| CategoryError::NotFound(format!("Category '{id}' not found")))?;

    if !existing.is_active {
        return Ok(()); // Already archived / idempotent
    }

    let active_children: i64 = conn.query_row(
        "SELECT COUNT(*) FROM categories WHERE parent_id = ?1 AND is_active = 1",
        [id],
        |row| row.get(0),
    )?;

    if active_children > 0 {
        return Err(CategoryError::HasActiveChildren(format!(
            "Cannot archive category '{id}' because it has {active_children} active subcategory(ies)"
        )));
    }

    conn.execute(
        "UPDATE categories SET
            is_active = 0,
            updated_at = datetime('now')
        WHERE id = ?1 AND is_active = 1",
        [id],
    )?;

    Ok(())
}

/// Lists categories matching the specified filter.
pub fn list_categories(
    conn: &Connection,
    filter: &CategoryFilter,
) -> Result<Vec<Category>, CategoryError> {
    let mut sql = format!("SELECT {CATEGORY_COLUMNS} FROM categories WHERE 1=1");
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(active) = filter.is_active {
        sql.push_str(" AND is_active = ?");
        params_vec.push(Box::new(if active { 1 } else { 0 }));
    }

    if let Some(ref p_opt) = filter.parent_id {
        match p_opt {
            None => {
                sql.push_str(" AND parent_id IS NULL");
            }
            Some(pid) => {
                let trimmed = pid.trim();
                if trimmed.is_empty() {
                    sql.push_str(" AND parent_id IS NULL");
                } else {
                    sql.push_str(" AND parent_id = ?");
                    params_vec.push(Box::new(trimmed.to_string()));
                }
            }
        }
    }

    crate::db::append_name_or_description_search(
        &mut sql,
        &mut params_vec,
        filter.query.as_deref(),
    );

    sql.push_str(" ORDER BY name COLLATE NOCASE ASC");

    let params_slice: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(AsRef::as_ref).collect();
    let mut stmt = conn.prepare(&sql)?;
    let category_iter = stmt.query_map(params_slice.as_slice(), map_category_row)?;

    let mut categories = Vec::new();
    for c in category_iter {
        categories.push(c?);
    }
    Ok(categories)
}

fn partition_categories(
    categories: Vec<Category>,
) -> (Vec<Category>, HashMap<String, Vec<Category>>) {
    let active_ids: HashSet<String> = categories.iter().map(|c| c.id.clone()).collect();
    let mut children_by_parent: HashMap<String, Vec<Category>> = HashMap::new();
    let mut root_categories: Vec<Category> = Vec::new();

    for cat in categories {
        let is_orphan_or_root = match cat.parent_id.as_ref() {
            None => true,
            Some(pid) => !active_ids.contains(pid),
        };

        if is_orphan_or_root {
            root_categories.push(cat);
        } else if let Some(pid) = cat.parent_id.clone() {
            children_by_parent.entry(pid).or_default().push(cat);
        }
    }
    (root_categories, children_by_parent)
}

fn build_category_tree_node(
    cat: Category,
    children_map: &mut HashMap<String, Vec<Category>>,
) -> CategoryTreeNode {
    let mut raw_children = children_map.remove(&cat.id).unwrap_or_default();
    raw_children.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    let children = raw_children
        .into_iter()
        .map(|child| build_category_tree_node(child, children_map))
        .collect();

    CategoryTreeNode {
        category: cat,
        children,
    }
}

fn drain_stranded_categories(
    mut children_map: HashMap<String, Vec<Category>>,
    tree: &mut Vec<CategoryTreeNode>,
) {
    while !children_map.is_empty() {
        let next_key = match children_map.keys().next() {
            Some(k) => k.clone(),
            None => break,
        };
        if let Some(stranded_list) = children_map.remove(&next_key) {
            for stranded in stranded_list {
                tree.push(build_category_tree_node(stranded, &mut children_map));
            }
        }
    }
}

/// Reconstructs the complete hierarchical category tree in memory in O(n) time.
pub fn get_category_tree(
    conn: &Connection,
    include_inactive: bool,
) -> Result<Vec<CategoryTreeNode>, CategoryError> {
    let filter = CategoryFilter {
        query: None,
        parent_id: None,
        is_active: if include_inactive { None } else { Some(true) },
    };

    let all_categories = list_categories(conn, &filter)?;
    let (mut root_categories, mut children_by_parent) = partition_categories(all_categories);

    root_categories.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let mut tree: Vec<CategoryTreeNode> = root_categories
        .into_iter()
        .map(|root| build_category_tree_node(root, &mut children_by_parent))
        .collect();

    drain_stranded_categories(children_by_parent, &mut tree);

    tree.sort_by(|a, b| {
        a.category
            .name
            .to_lowercase()
            .cmp(&b.category.name.to_lowercase())
    });

    Ok(tree)
}
