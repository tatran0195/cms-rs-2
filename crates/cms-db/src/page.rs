//! Page database queries
//!
//! This module contains all database queries related to pages,
//! including the page tree structure with parent/child relationships.

use chrono::{DateTime, Utc};
use cms_entity::page::{Page, PageListItem, PageTreeNode};
use cms_error::AppError;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

/// Database representation of a page row
#[derive(Debug, FromRow)]
struct PageRow {
    id: String,
    project_id: String,
    branch_id: String,
    language_id: Option<String>,
    parent_id: Option<String>,
    kind: Option<String>,
    path: String,
    slug: String,
    title: String,
    description: Option<String>,
    content: String,
    icon: Option<String>,
    config: Option<serde_json::Value>,
    translation_key: Option<String>,
    position: i32,
    is_published: bool,
    is_indexed: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Database representation of a page tree node for hierarchical queries
#[derive(Debug, FromRow)]
struct PageTreeRow {
    id: String,
    project_id: String,
    branch_id: String,
    parent_id: Option<String>,
    path: String,
    slug: String,
    title: String,
    position: i32,
    is_published: bool,
}

#[derive(Debug, FromRow)]
struct PageReorderRow {
    id: String,
    parent_id: Option<String>,
    slug: String,
    position: i32,
}

impl From<PageRow> for Page {
    fn from(row: PageRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            branch_id: row.branch_id,
            language_id: row.language_id,
            parent_id: row.parent_id,
            kind: row.kind.unwrap_or_else(|| "PAGE".to_string()),
            path: row.path,
            slug: row.slug,
            title: row.title,
            description: row.description,
            content: row.content,
            icon: row.icon,
            config: row.config,
            translation_key: row.translation_key,
            position: row.position,
            is_published: row.is_published,
            is_indexed: row.is_indexed,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<PageRow> for PageListItem {
    fn from(row: PageRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            branch_id: row.branch_id,
            parent_id: row.parent_id,
            language_id: row.language_id,
            kind: row.kind.or(Some("PAGE".to_string())),
            path: row.path,
            slug: row.slug,
            title: row.title,
            description: row.description,
            content: Some(row.content),
            icon: row.icon,
            config: row.config,
            translation_key: row.translation_key,
            position: row.position,
            is_published: row.is_published,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Page queries
pub struct PageQueries;

impl PageQueries {
    /// Get published PAGE records for one project/branch across all languages.
    /// GROUP nodes remain in the navigation tree but are not rendered as documents.
    pub async fn get_deployable_by_project_branch(
        pool: &PgPool,
        project_id: &str,
        branch_id: &str,
    ) -> Result<Vec<Page>, AppError> {
        let rows = sqlx::query_as::<_, PageRow>(
            r#"
            SELECT * FROM "Page"
            WHERE project_id = $1
              AND branch_id = $2
              AND is_published = TRUE
              AND UPPER(COALESCE(kind, 'PAGE')) = 'PAGE'
            ORDER BY language_id, path
            "#,
        )
        .bind(project_id)
        .bind(branch_id)
        .fetch_all(pool)
        .await
        .map_err(|error| AppError::Database(error.into()))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Get a page by ID
    pub async fn get_by_id(pool: &PgPool, page_id: &str) -> Result<Option<Page>, AppError> {
        let row = sqlx::query_as::<_, PageRow>("SELECT * FROM \"Page\" WHERE id = $1")
            .bind(page_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get just the path of a page by ID (lightweight for slug dedup).
    pub async fn get_path(pool: &PgPool, page_id: &str) -> Result<Option<String>, AppError> {
        let path: Option<String> = sqlx::query_scalar("SELECT path FROM \"Page\" WHERE id = $1")
            .bind(page_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;
        Ok(path)
    }

    /// Find a page using the complete public scope. Paths are only unique inside
    /// one project/branch/language, so public readers must never use the legacy
    /// project+branch-only lookup for multilingual content.
    pub async fn get_by_project_branch_language_and_path(
        pool: &PgPool,
        project_id: &str,
        branch_id: &str,
        language_id: &str,
        path: &str,
        published_only: bool,
    ) -> Result<Option<Page>, AppError> {
        let row = sqlx::query_as::<_, PageRow>(
            r#"
            SELECT * FROM "Page"
            WHERE project_id = $1 AND branch_id = $2 AND language_id = $3 AND path = $4
              AND ($5 = FALSE OR is_published = TRUE)
            "#,
        )
        .bind(project_id)
        .bind(branch_id)
        .bind(language_id)
        .bind(path)
        .bind(published_only)
        .fetch_optional(pool)
        .await?;
        Ok(row.map(Into::into))
    }

    /// Load full page records for one language/version. This is intended for
    /// public-site assembly; it avoids mixing other languages or branches.
    pub async fn get_by_project_branch_language_full(
        pool: &PgPool,
        project_id: &str,
        branch_id: &str,
        language_id: &str,
        published_only: bool,
    ) -> Result<Vec<Page>, AppError> {
        let rows = sqlx::query_as::<_, PageRow>(
            r#"
            SELECT * FROM "Page"
            WHERE project_id = $1 AND branch_id = $2 AND language_id = $3
              AND ($4 = FALSE OR is_published = TRUE)
            ORDER BY position ASC, created_at ASC, id ASC
            "#,
        )
        .bind(project_id)
        .bind(branch_id)
        .bind(language_id)
        .bind(published_only)
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Load published pages that represent one page in other enabled languages.
    /// Translation keys are preferred; pages without a key fall back to matching
    /// their path, preserving compatibility with older content.
    pub async fn get_published_alternates(
        pool: &PgPool,
        project_id: &str,
        branch_id: &str,
        language_ids: &[String],
        translation_key: Option<&str>,
        path: &str,
    ) -> Result<Vec<Page>, AppError> {
        if language_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut query = QueryBuilder::<Postgres>::new(
            "SELECT * FROM \"Page\" WHERE project_id = ",
        );
        query.push_bind(project_id);
        query.push(" AND branch_id = ");
        query.push_bind(branch_id);
        query.push(" AND is_published = TRUE AND language_id IN (");
        let mut separated = query.separated(", ");
        for language_id in language_ids {
            separated.push_bind(language_id);
        }
        separated.push_unseparated(")");
        if let Some(translation_key) = translation_key {
            query.push(" AND (translation_key = ");
            query.push_bind(translation_key);
            query.push(" OR (translation_key IS NULL AND path = ");
            query.push_bind(path);
            query.push("))");
        } else {
            query.push(" AND path = ");
            query.push_bind(path);
        }
        query.push(" ORDER BY language_id, created_at ASC, id ASC");
        let rows = query.build_query_as::<PageRow>().fetch_all(pool).await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Get a page by path
    pub async fn get_by_path(
        pool: &PgPool,
        project_id: &str,
        branch_id: &str,
        path: &str,
    ) -> Result<Option<Page>, AppError> {
        let row = sqlx::query_as::<_, PageRow>(
            "SELECT * FROM \"Page\" WHERE project_id = $1 AND branch_id = $2 AND path = $3",
        )
        .bind(project_id)
        .bind(branch_id)
        .bind(path)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get pages by project and branch
    pub async fn get_by_project_and_branch(
        pool: &PgPool,
        project_id: &str,
        branch_id: &str,
        parent_id: Option<&str>,
        is_published: Option<bool>,
        search: Option<&str>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<PageListItem>, AppError> {
        Self::get_by_project_branch_and_language(
            pool,
            project_id,
            branch_id,
            None,
            parent_id,
            is_published,
            search,
            limit,
            offset,
        )
        .await
    }

    /// Get pages by project, branch, and optional language
    pub async fn get_by_project_branch_and_language(
        pool: &PgPool,
        project_id: &str,
        branch_id: &str,
        language_id: Option<&str>,
        parent_id: Option<&str>,
        is_published: Option<bool>,
        search: Option<&str>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<PageListItem>, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT * FROM \"Page\" WHERE project_id = ");
        query_builder.push_bind(project_id);
        query_builder.push(" AND branch_id = ");
        query_builder.push_bind(branch_id);

        if let Some(lang_id) = language_id {
            if !lang_id.is_empty() {
                query_builder.push(" AND language_id = ");
                query_builder.push_bind(lang_id);
            }
        }

        if let Some(parent_id) = parent_id {
            if parent_id.is_empty() {
                query_builder.push(" AND parent_id IS NULL");
            } else {
                query_builder.push(" AND parent_id = ");
                query_builder.push_bind(parent_id);
            }
        }

        if let Some(is_published) = is_published {
            query_builder.push(" AND is_published = ");
            query_builder.push_bind(is_published);
        }

        if let Some(search) = search {
            query_builder.push(" AND (title ILIKE ");
            query_builder.push_bind(format!("%{}%", search));
            query_builder.push(" OR content ILIKE ");
            query_builder.push_bind(format!("%{}%", search));
            query_builder.push(")");
        }

        query_builder.push(" ORDER BY position ASC, created_at DESC");

        if let Some(limit) = limit {
            query_builder.push(" LIMIT ");
            query_builder.push_bind(limit);
        }

        if let Some(offset) = offset {
            query_builder.push(" OFFSET ");
            query_builder.push_bind(offset);
        }

        let rows = query_builder
            .build_query_as::<PageRow>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(PageListItem::from).collect())
    }

    /// Count pages by project and branch
    pub async fn count_by_project_and_branch(
        pool: &PgPool,
        project_id: &str,
        branch_id: &str,
        parent_id: Option<&str>,
        is_published: Option<bool>,
        search: Option<&str>,
    ) -> Result<i64, AppError> {
        Self::count_by_project_branch_and_language(
            pool,
            project_id,
            branch_id,
            None,
            parent_id,
            is_published,
            search,
        )
        .await
    }

    /// Count pages by project, branch, and optional language
    pub async fn count_by_project_branch_and_language(
        pool: &PgPool,
        project_id: &str,
        branch_id: &str,
        language_id: Option<&str>,
        parent_id: Option<&str>,
        is_published: Option<bool>,
        search: Option<&str>,
    ) -> Result<i64, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT COUNT(*) as count FROM \"Page\" WHERE project_id = ");
        query_builder.push_bind(project_id);
        query_builder.push(" AND branch_id = ");
        query_builder.push_bind(branch_id);

        if let Some(lang_id) = language_id {
            if !lang_id.is_empty() {
                query_builder.push(" AND language_id = ");
                query_builder.push_bind(lang_id);
            }
        }

        if let Some(parent_id) = parent_id {
            if parent_id.is_empty() {
                query_builder.push(" AND parent_id IS NULL");
            } else {
                query_builder.push(" AND parent_id = ");
                query_builder.push_bind(parent_id);
            }
        }

        if let Some(is_published) = is_published {
            query_builder.push(" AND is_published = ");
            query_builder.push_bind(is_published);
        }

        if let Some(search) = search {
            query_builder.push(" AND (title ILIKE ");
            query_builder.push_bind(format!("%{}%", search));
            query_builder.push(" OR content ILIKE ");
            query_builder.push_bind(format!("%{}%", search));
            query_builder.push(")");
        }

        let count: i64 = query_builder
            .build()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?
            .get::<i64, _>("count");

        Ok(count)
    }

    /// Get the full page tree for a project and branch
    pub async fn get_tree(
        pool: &PgPool,
        project_id: &str,
        branch_id: &str,
        is_published: Option<bool>,
    ) -> Result<Vec<PageTreeNode>, AppError> {
        // Get all pages that match the criteria
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT id, project_id, branch_id, parent_id, path, slug, title, position, \
             is_published FROM \"Page\" WHERE project_id = ",
        );
        query_builder.push_bind(project_id);
        query_builder.push(" AND branch_id = ");
        query_builder.push_bind(branch_id);

        if let Some(is_published) = is_published {
            query_builder.push(" AND is_published = ");
            query_builder.push_bind(is_published);
        }

        query_builder.push(" ORDER BY position ASC");

        let rows = query_builder
            .build_query_as::<PageTreeRow>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        // Build the tree structure from flat rows
        let mut nodes: Vec<PageTreeNode> = rows
            .into_iter()
            .map(|r| PageTreeNode {
                id: r.id,
                project_id: r.project_id,
                branch_id: r.branch_id,
                parent_id: r.parent_id,
                path: r.path,
                slug: r.slug,
                title: r.title,
                position: r.position,
                is_published: r.is_published,
                has_children: false,
                children: None,
            })
            .collect();

        // Build parent→children map using node IDs
        let mut parent_children: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        let mut root_ids: Vec<String> = Vec::new();

        for node in &nodes {
            if let Some(parent_id) = &node.parent_id {
                parent_children
                    .entry(parent_id.clone())
                    .or_default()
                    .push(node.id.clone());
            } else {
                root_ids.push(node.id.clone());
            }
        }

        // Convert nodes to a HashMap for lookup
        let mut node_lookup: std::collections::HashMap<String, PageTreeNode> =
            nodes.into_iter().map(|n| (n.id.clone(), n)).collect();

        // Mark nodes that have children
        for (parent_id, children) in &parent_children {
            if let Some(parent) = node_lookup.get_mut(parent_id) {
                parent.has_children = true;
                parent.children = Some(Vec::new());
            }
        }

        // Recursive function to attach children
        fn attach_children(
            id: &str,
            node_lookup: &mut std::collections::HashMap<String, PageTreeNode>,
            parent_children: &std::collections::HashMap<String, Vec<String>>,
        ) -> Option<PageTreeNode> {
            let mut node = node_lookup.remove(id)?;
            if let Some(child_ids) = parent_children.get(id) {
                let mut children: Vec<PageTreeNode> = child_ids
                    .iter()
                    .filter_map(|cid| attach_children(cid, node_lookup, parent_children))
                    .collect();
                children.sort_by_key(|a| a.position);
                node.children = Some(children);
                node.has_children = true;
            }
            Some(node)
        }

        let mut root_nodes: Vec<PageTreeNode> = root_ids
            .iter()
            .filter_map(|id| attach_children(id, &mut node_lookup, &parent_children))
            .collect();

        // Add any orphaned nodes to root
        root_nodes.extend(node_lookup.into_values());

        // Sort children by position
        fn sort_children(node: &mut PageTreeNode) {
            if let Some(children) = &mut node.children {
                children.sort_by_key(|a| a.position);
                for child in children {
                    sort_children(child);
                }
            }
        }

        for node in &mut root_nodes {
            sort_children(node);
        }

        Ok(root_nodes)
    }

    /// Create a new page. Branch/language/parent scope is rechecked inside the
    /// insert transaction so concurrent moves cannot create a cross-tree edge.
    pub async fn create(
        pool: &PgPool,
        project_id: &str,
        branch_id: &str,
        language_id: Option<&str>,
        parent_id: Option<&str>,
        kind: Option<&str>,
        slug: &str,
        title: &str,
        description: Option<&str>,
        content: Option<&str>,
        icon: Option<&str>,
        config: Option<&serde_json::Value>,
        translation_key: Option<&str>,
        position: i32,
        is_published: bool,
    ) -> Result<Page, AppError> {
        let mut tx = pool.begin().await?;
        let branch_project = sqlx::query_scalar::<_, String>(
            "SELECT project_id FROM \"Branch\" WHERE id = $1 FOR KEY SHARE",
        )
        .bind(branch_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Branch not found".to_string()))?;
        if branch_project != project_id {
            return Err(AppError::Conflict(
                "Branch does not belong to this project".to_string(),
            ));
        }

        if let Some(language_id) = language_id {
            let language_project = sqlx::query_scalar::<_, String>(
                "SELECT project_id FROM \"Language\" WHERE id = $1 FOR KEY SHARE",
            )
            .bind(language_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Language not found".to_string()))?;
            if language_project != project_id {
                return Err(AppError::Conflict(
                    "Language does not belong to this project".to_string(),
                ));
            }
        }

        let effective_parent_id = parent_id.filter(|id| !id.is_empty());
        let path = if let Some(parent_id) = effective_parent_id {
            let parent = sqlx::query_as::<_, (String, String, Option<String>, String)>(
                "SELECT project_id, branch_id, language_id, path FROM \"Page\" WHERE id = $1 FOR KEY SHARE",
            )
            .bind(parent_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Parent page not found".to_string()))?;
            if parent.0 != project_id || parent.1 != branch_id || parent.2.as_deref() != language_id
            {
                return Err(AppError::Conflict(
                    "Parent page must belong to the same project, branch, and language".to_string(),
                ));
            }
            format!("{}/{}", parent.3.trim_end_matches('/'), slug)
        } else {
            format!("/{slug}")
        };

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let effective_kind = kind.unwrap_or("PAGE");
        let row = sqlx::query_as::<_, PageRow>(
            r#"
            INSERT INTO "Page" (
                id, project_id, branch_id, language_id, parent_id, kind,
                path, slug, title, description, content, icon, config,
                translation_key, position, is_published, is_indexed, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(branch_id)
        .bind(language_id)
        .bind(effective_parent_id)
        .bind(effective_kind)
        .bind(&path)
        .bind(slug)
        .bind(title)
        .bind(description)
        .bind(content.unwrap_or(""))
        .bind(icon)
        .bind(config)
        .bind(translation_key)
        .bind(position)
        .bind(is_published)
        .bind(is_published)
        .bind(now)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_page_write_error)?;

        tx.commit().await?;
        Ok(row.into())
    }

    /// Update a page and atomically recompute materialized paths for every
    /// descendant. A slug/parent change that collides anywhere in the moved
    /// subtree fails without partially changing the tree.
    pub async fn update(
        pool: &PgPool,
        page_id: &str,
        parent_id: Option<&str>,
        language_id: Option<&str>,
        kind: Option<&str>,
        slug: Option<&str>,
        title: Option<&str>,
        description: Option<&str>,
        content: Option<&str>,
        icon: Option<&str>,
        config: Option<Option<&serde_json::Value>>,
        translation_key: Option<&str>,
        position: Option<i32>,
        is_published: Option<bool>,
    ) -> Result<Page, AppError> {
        let mut tx = pool.begin().await?;
        let current =
            sqlx::query_as::<_, PageRow>("SELECT * FROM \"Page\" WHERE id = $1 FOR UPDATE")
                .bind(page_id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;

        let target_language_id = match language_id {
            Some("") => None,
            Some(language_id) => Some(language_id),
            None => current.language_id.as_deref(),
        };
        if let Some(language_id) = target_language_id {
            let language_project = sqlx::query_scalar::<_, String>(
                "SELECT project_id FROM \"Language\" WHERE id = $1 FOR KEY SHARE",
            )
            .bind(language_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Language not found".to_string()))?;
            if language_project != current.project_id {
                return Err(AppError::Conflict(
                    "Language does not belong to this project".to_string(),
                ));
            }
        }

        let target_parent_id = match parent_id {
            Some("") => None,
            Some(parent_id) => Some(parent_id),
            None => current.parent_id.as_deref(),
        };
        let parent_path = if let Some(parent_id) = target_parent_id {
            let would_cycle = sqlx::query_scalar::<_, bool>(
                r#"
                WITH RECURSIVE descendants (id, visited, depth) AS (
                    SELECT id, ARRAY[id]::TEXT[], 0 FROM "Page" WHERE id = $1
                    UNION ALL
                    SELECT child.id, descendants.visited || child.id, descendants.depth + 1
                    FROM "Page" AS child
                    JOIN descendants ON child.parent_id = descendants.id
                    WHERE descendants.depth < 256
                      AND NOT child.id = ANY(descendants.visited)
                )
                SELECT EXISTS(SELECT 1 FROM descendants WHERE id = $2)
                "#,
            )
            .bind(page_id)
            .bind(parent_id)
            .fetch_one(&mut *tx)
            .await?;
            if would_cycle {
                return Err(AppError::Conflict(
                    "Cannot move a page under itself or one of its descendants".to_string(),
                ));
            }
            let parent = sqlx::query_as::<_, (String, String, Option<String>, String)>(
                "SELECT project_id, branch_id, language_id, path FROM \"Page\" WHERE id = $1 FOR KEY SHARE",
            )
            .bind(parent_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Parent page not found".to_string()))?;
            if parent.0 != current.project_id
                || parent.1 != current.branch_id
                || parent.2.as_deref() != target_language_id
            {
                return Err(AppError::Conflict(
                    "Parent page must belong to the same project, branch, and language".to_string(),
                ));
            }
            Some(parent.3)
        } else {
            None
        };
        let effective_slug = slug.unwrap_or(&current.slug);
        let new_path = match parent_path.as_deref() {
            Some(parent_path) => {
                format!("{}/{}", parent_path.trim_end_matches('/'), effective_slug)
            }
            None => format!("/{effective_slug}"),
        };
        if new_path.len() > 1024 {
            return Err(AppError::Validation(
                "Page path must be 1024 bytes or fewer".to_string(),
            ));
        }

        // Descendants must remain inside one project/branch/language tree. A
        // language move with children is rejected rather than silently changing
        // only part of the subtree.
        let tree_state = sqlx::query_as::<_, (bool, bool, bool)>(
            r#"
            WITH RECURSIVE subtree (id, project_id, branch_id, language_id, depth, visited) AS (
                SELECT id, project_id, branch_id, language_id, 0, ARRAY[id]::TEXT[]
                FROM "Page" WHERE id = $1
                UNION ALL
                SELECT child.id, child.project_id, child.branch_id, child.language_id,
                       subtree.depth + 1, subtree.visited || child.id
                FROM "Page" AS child
                JOIN subtree ON child.parent_id = subtree.id
                WHERE subtree.depth < 256 AND NOT child.id = ANY(subtree.visited)
            )
            SELECT
                EXISTS (
                    SELECT 1 FROM subtree
                    WHERE id <> $1
                      AND (project_id <> $2 OR branch_id <> $3 OR language_id IS DISTINCT FROM $4)
                ),
                EXISTS (
                    SELECT 1 FROM subtree
                    JOIN "Page" AS child ON child.parent_id = subtree.id
                    WHERE child.id = ANY(subtree.visited)
                ),
                EXISTS (
                    SELECT 1 FROM subtree
                    JOIN "Page" AS child ON child.parent_id = subtree.id
                    WHERE subtree.depth >= 256 AND NOT child.id = ANY(subtree.visited)
                )
            "#,
        )
        .bind(page_id)
        .bind(&current.project_id)
        .bind(&current.branch_id)
        .bind(target_language_id)
        .fetch_one(&mut *tx)
        .await?;
        if tree_state.0 {
            return Err(AppError::Conflict(
                "All descendants must share the page's project, branch, and language".to_string(),
            ));
        }
        if tree_state.1 {
            return Err(AppError::Conflict(
                "The page tree contains a parent cycle".to_string(),
            ));
        }
        if tree_state.2 {
            return Err(AppError::Validation(
                "Page trees cannot exceed 256 levels".to_string(),
            ));
        }

        let (path_collision, path_too_long) = sqlx::query_as::<_, (bool, bool)>(
            r#"
            WITH RECURSIVE planned (id, path, depth) AS (
                SELECT $1::TEXT, $2::TEXT, 0
                UNION ALL
                SELECT child.id,
                       RTRIM(planned.path, '/') || '/' || child.slug,
                       planned.depth + 1
                FROM "Page" AS child
                JOIN planned ON child.parent_id = planned.id
                WHERE child.project_id = $3 AND child.branch_id = $4
                  AND child.language_id IS NOT DISTINCT FROM $5
                  AND planned.depth < 256
            )
            SELECT
                EXISTS (
                    SELECT 1 FROM planned
                    JOIN "Page" AS occupied
                      ON occupied.project_id = $3
                     AND occupied.branch_id = $4
                     AND occupied.language_id IS NOT DISTINCT FROM $5
                     AND occupied.path = planned.path
                     AND occupied.id <> planned.id
                ),
                EXISTS (SELECT 1 FROM planned WHERE OCTET_LENGTH(path) > 1024)
            "#,
        )
        .bind(page_id)
        .bind(&new_path)
        .bind(&current.project_id)
        .bind(&current.branch_id)
        .bind(target_language_id)
        .fetch_one(&mut *tx)
        .await?;
        if path_too_long {
            return Err(AppError::Validation(
                "A descendant page path would exceed 1024 bytes".to_string(),
            ));
        }
        if path_collision {
            return Err(AppError::Conflict(
                "Page path change conflicts with an existing page".to_string(),
            ));
        }

        let mut query = QueryBuilder::<Postgres>::new("UPDATE \"Page\" SET ");
        let mut has_updates = false;
        if let Some(parent_id) = parent_id {
            if parent_id.is_empty() {
                query.push("parent_id = NULL");
            } else {
                query.push("parent_id = ");
                query.push_bind(parent_id);
            }
            has_updates = true;
        }
        if let Some(language_id) = language_id {
            if language_id.is_empty() {
                push_page_update_separator(&mut query, &mut has_updates);
                query.push("language_id = NULL");
            } else {
                push_page_update_separator(&mut query, &mut has_updates);
                query.push("language_id = ");
                query.push_bind(language_id);
            }
        }
        if let Some(kind) = kind {
            push_page_update_separator(&mut query, &mut has_updates);
            query.push("kind = ");
            query.push_bind(kind);
        }
        if let Some(slug) = slug {
            push_page_update_separator(&mut query, &mut has_updates);
            query.push("slug = ");
            query.push_bind(slug);
        }
        if let Some(title) = title {
            push_page_update_separator(&mut query, &mut has_updates);
            query.push("title = ");
            query.push_bind(title);
        }
        if let Some(description) = description {
            push_page_update_separator(&mut query, &mut has_updates);
            query.push("description = ");
            query.push_bind(description);
        }
        if let Some(content) = content {
            push_page_update_separator(&mut query, &mut has_updates);
            query.push("content = ");
            query.push_bind(content);
        }
        if let Some(icon) = icon {
            push_page_update_separator(&mut query, &mut has_updates);
            query.push("icon = ");
            query.push_bind(icon);
        }
        if let Some(config) = config {
            push_page_update_separator(&mut query, &mut has_updates);
            query.push("config = ");
            query.push_bind(config);
        }
        if let Some(translation_key) = translation_key {
            push_page_update_separator(&mut query, &mut has_updates);
            query.push("translation_key = ");
            query.push_bind(translation_key);
        }
        if let Some(position) = position {
            push_page_update_separator(&mut query, &mut has_updates);
            query.push("position = ");
            query.push_bind(position);
        }
        if let Some(is_published) = is_published {
            push_page_update_separator(&mut query, &mut has_updates);
            query.push("is_published = ");
            query.push_bind(is_published);
            query.push(", is_indexed = ");
            query.push_bind(is_published);
        }
        push_page_update_separator(&mut query, &mut has_updates);
        query.push("updated_at = ");
        query.push_bind(Utc::now());
        query.push(" WHERE id = ");
        query.push_bind(page_id);

        query
            .build()
            .execute(&mut *tx)
            .await
            .map_err(map_page_write_error)?;

        sqlx::query(
            r#"
            WITH RECURSIVE paths (id, path, depth) AS (
                SELECT $1::TEXT, $2::TEXT, 0
                UNION ALL
                SELECT child.id,
                       RTRIM(paths.path, '/') || '/' || child.slug,
                       paths.depth + 1
                FROM "Page" AS child
                JOIN paths ON child.parent_id = paths.id
                WHERE child.project_id = $3 AND child.branch_id = $4
                  AND child.language_id IS NOT DISTINCT FROM $5
                  AND paths.depth < 256
            )
            UPDATE "Page" AS page
            SET path = paths.path, updated_at = $6
            FROM paths
            WHERE page.id = paths.id
            "#,
        )
        .bind(page_id)
        .bind(&new_path)
        .bind(&current.project_id)
        .bind(&current.branch_id)
        .bind(target_language_id)
        .bind(Utc::now())
        .execute(&mut *tx)
        .await
        .map_err(map_page_write_error)?;

        let updated = sqlx::query_as::<_, PageRow>("SELECT * FROM \"Page\" WHERE id = $1")
            .bind(page_id)
            .fetch_one(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(updated.into())
    }

    /// Delete a page while preserving its children as root pages. The FK uses
    /// ON DELETE SET NULL; this transaction also recalculates every descendant
    /// path and rejects collisions before any tree state is committed.
    pub async fn delete(pool: &PgPool, page_id: &str) -> Result<bool, AppError> {
        let mut tx = pool.begin().await?;
        let Some(current) = sqlx::query_as::<_, PageRow>(
            "SELECT * FROM \"Page\" WHERE id = $1 FOR UPDATE",
        )
        .bind(page_id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(false);
        };

        let tree_state = sqlx::query_as::<_, (bool, bool, bool)>(
            r#"
            WITH RECURSIVE subtree (id, project_id, branch_id, language_id, depth, visited) AS (
                SELECT id, project_id, branch_id, language_id, 0, ARRAY[id]::TEXT[]
                FROM "Page" WHERE id = $1
                UNION ALL
                SELECT child.id, child.project_id, child.branch_id, child.language_id,
                       subtree.depth + 1, subtree.visited || child.id
                FROM "Page" AS child
                JOIN subtree ON child.parent_id = subtree.id
                WHERE subtree.depth < 256 AND NOT child.id = ANY(subtree.visited)
            )
            SELECT
                EXISTS (
                    SELECT 1 FROM subtree
                    WHERE project_id <> $2 OR branch_id <> $3
                       OR language_id IS DISTINCT FROM $4
                ),
                EXISTS (
                    SELECT 1 FROM subtree
                    JOIN "Page" AS child ON child.parent_id = subtree.id
                    WHERE child.id = ANY(subtree.visited)
                ),
                EXISTS (
                    SELECT 1 FROM subtree
                    JOIN "Page" AS child ON child.parent_id = subtree.id
                    WHERE subtree.depth >= 256 AND NOT child.id = ANY(subtree.visited)
                )
            "#,
        )
        .bind(page_id)
        .bind(&current.project_id)
        .bind(&current.branch_id)
        .bind(current.language_id.as_deref())
        .fetch_one(&mut *tx)
        .await?;
        if tree_state.0 {
            return Err(AppError::Conflict(
                "All descendants must share the deleted page's project, branch, and language".to_string(),
            ));
        }
        if tree_state.1 {
            return Err(AppError::Conflict(
                "The page tree contains a parent cycle".to_string(),
            ));
        }
        if tree_state.2 {
            return Err(AppError::Validation(
                "Page trees cannot exceed 256 levels".to_string(),
            ));
        }

        let children = sqlx::query_as::<_, (String, String, i32, DateTime<Utc>)>(
            r#"SELECT id, slug, position, created_at FROM "Page"
               WHERE parent_id = $1 ORDER BY position, created_at, id FOR UPDATE"#,
        )
        .bind(page_id)
        .fetch_all(&mut *tx)
        .await?;
        if children.is_empty() {
            let result = sqlx::query("DELETE FROM \"Page\" WHERE id = $1")
                .bind(page_id)
                .execute(&mut *tx)
                .await
                .map_err(map_page_write_error)?;
            tx.commit().await?;
            return Ok(result.rows_affected() > 0);
        }

        // Check all paths produced by promoting the direct children to roots.
        // A collision returns a clear conflict instead of leaving SET NULL
        // children with stale materialized paths.
        let (path_too_long, path_collision, duplicate_planned_path) =
            sqlx::query_as::<_, (bool, bool, bool)>(
                r#"
                WITH RECURSIVE planned (id, path, visited, depth) AS (
                    SELECT id, '/' || slug, ARRAY[id]::TEXT[], 0
                    FROM "Page" WHERE parent_id = $1
                    UNION ALL
                    SELECT child.id,
                           RTRIM(planned.path, '/') || '/' || child.slug,
                           planned.visited || child.id,
                           planned.depth + 1
                    FROM "Page" AS child
                    JOIN planned ON child.parent_id = planned.id
                    WHERE planned.depth < 256 AND NOT child.id = ANY(planned.visited)
                )
                SELECT
                    EXISTS (SELECT 1 FROM planned WHERE OCTET_LENGTH(path) > 1024),
                    EXISTS (
                        SELECT 1 FROM planned
                        JOIN "Page" AS occupied
                          ON occupied.project_id = $2
                         AND occupied.branch_id = $3
                         AND occupied.language_id IS NOT DISTINCT FROM $4
                         AND occupied.path = planned.path
                         AND occupied.id <> planned.id
                        WHERE occupied.id <> $1
                          AND NOT EXISTS (
                              SELECT 1 FROM planned AS moving WHERE moving.id = occupied.id
                          )
                    ),
                    EXISTS (
                        SELECT path FROM planned GROUP BY path HAVING COUNT(*) > 1
                    )
                "#,
            )
            .bind(page_id)
            .bind(&current.project_id)
            .bind(&current.branch_id)
            .bind(current.language_id.as_deref())
            .fetch_one(&mut *tx)
            .await?;
        if path_too_long {
            return Err(AppError::Validation(
                "A reparented page path would exceed 1024 bytes".to_string(),
            ));
        }
        if path_collision || duplicate_planned_path {
            return Err(AppError::Conflict(
                "Deleting this page would create a path collision; rename or move its children first".to_string(),
            ));
        }

        let next_position: i64 = sqlx::query_scalar(
            r#"SELECT COALESCE(MAX(position)::BIGINT + 1, 0) FROM "Page"
               WHERE project_id = $1 AND branch_id = $2
                 AND language_id IS NOT DISTINCT FROM $3
                 AND parent_id IS NULL AND id <> $4"#,
        )
        .bind(&current.project_id)
        .bind(&current.branch_id)
        .bind(current.language_id.as_deref())
        .bind(page_id)
        .fetch_one(&mut *tx)
        .await?;
        if next_position + children.len() as i64 - 1 > i32::MAX as i64 {
            return Err(AppError::Validation(
                "Page position limit reached while reparenting children".to_string(),
            ));
        }

        // Remove the parent first so its ON DELETE SET NULL action can run; the
        // complete operation remains in this transaction and rolls back on any
        // later error.
        sqlx::query("DELETE FROM \"Page\" WHERE id = $1")
            .bind(page_id)
            .execute(&mut *tx)
            .await
            .map_err(map_page_write_error)?;

        for (index, (child_id, slug, _, _)) in children.iter().enumerate() {
            sqlx::query(
                r#"UPDATE "Page"
                   SET parent_id = NULL, path = $1, position = $2, updated_at = $3
                   WHERE id = $4"#,
            )
            .bind(format!("/{slug}"))
            .bind((next_position + index as i64) as i32)
            .bind(Utc::now())
            .bind(child_id)
            .execute(&mut *tx)
            .await
            .map_err(map_page_write_error)?;
        }

        let child_ids = children
            .iter()
            .map(|(id, _, _, _)| id.clone())
            .collect::<Vec<_>>();
        sqlx::query(
            r#"
            WITH RECURSIVE paths (id, path, visited, depth) AS (
                SELECT page.id, '/' || page.slug, ARRAY[page.id]::TEXT[], 0
                FROM "Page" AS page
                WHERE page.id = ANY($1)
                UNION ALL
                SELECT child.id,
                       RTRIM(paths.path, '/') || '/' || child.slug,
                       paths.visited || child.id,
                       paths.depth + 1
                FROM "Page" AS child
                JOIN paths ON child.parent_id = paths.id
                WHERE child.project_id = $2 AND child.branch_id = $3
                  AND child.language_id IS NOT DISTINCT FROM $4
                  AND paths.depth < 256
                  AND NOT child.id = ANY(paths.visited)
            )
            UPDATE "Page" AS page
            SET path = paths.path, updated_at = $5
            FROM paths
            WHERE page.id = paths.id
            "#,
        )
        .bind(&child_ids)
        .bind(&current.project_id)
        .bind(&current.branch_id)
        .bind(current.language_id.as_deref())
        .bind(Utc::now())
        .execute(&mut *tx)
        .await
        .map_err(map_page_write_error)?;

        tx.commit().await?;
        Ok(true)
    }

    /// Atomically apply a tree reorder inside one project/branch/language scope.
    /// The transaction locks the scope, validates the proposed parent graph,
    /// calculates every materialized path, and uses temporary paths so unique
    /// constraints do not make harmless path swaps order-dependent.
    pub async fn reorder_tree(
        pool: &PgPool,
        project_id: &str,
        items: &[(String, Option<String>, i32)],
    ) -> Result<Vec<Page>, AppError> {
        if items.is_empty() {
            return Ok(Vec::new());
        }
        let mut seen_ids = std::collections::HashSet::new();
        for (id, _, position) in items {
            if !seen_ids.insert(id.as_str()) {
                return Err(AppError::Validation(
                    "A page may appear only once in a reorder request".to_string(),
                ));
            }
            if !(0..=1_000_000).contains(position) {
                return Err(AppError::Validation(
                    "Page position must be between 0 and 1000000".to_string(),
                ));
            }
        }

        let mut tx = pool.begin().await?;
        let first_id = &items[0].0;
        let (branch_id, language_id) = sqlx::query_as::<_, (String, Option<String>)>(
            r#"SELECT branch_id, language_id FROM "Page"
               WHERE id = $1 AND project_id = $2 FOR UPDATE"#,
        )
        .bind(first_id)
        .bind(project_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Page not found in this project".to_string()))?;

        let scope_rows = sqlx::query_as::<_, PageReorderRow>(
            r#"SELECT id, parent_id, slug, position FROM "Page"
               WHERE project_id = $1 AND branch_id = $2
                 AND language_id IS NOT DISTINCT FROM $3
               FOR UPDATE"#,
        )
        .bind(project_id)
        .bind(&branch_id)
        .bind(language_id.as_deref())
        .fetch_all(&mut *tx)
        .await?;
        let nodes = scope_rows
            .into_iter()
            .map(|row| (row.id.clone(), row))
            .collect::<std::collections::HashMap<_, _>>();
        let requested_ids = items.iter().map(|(id, _, _)| id.clone()).collect::<Vec<_>>();
        if items.iter().any(|(id, _, _)| !nodes.contains_key(id)) {
            return Err(AppError::Conflict(
                "All reordered pages must share a project, branch, and language".to_string(),
            ));
        }

        let mut parents = nodes
            .iter()
            .map(|(id, row)| (id.clone(), row.parent_id.clone()))
            .collect::<std::collections::HashMap<_, _>>();
        let mut positions = nodes
            .iter()
            .map(|(id, row)| (id.clone(), row.position))
            .collect::<std::collections::HashMap<_, _>>();
        for (id, parent_id, position) in items {
            if parent_id.as_deref() == Some(id.as_str()) {
                return Err(AppError::Conflict("A page cannot be its own parent".to_string()));
            }
            if parent_id
                .as_deref()
                .is_some_and(|parent_id| !nodes.contains_key(parent_id))
            {
                return Err(AppError::Conflict(
                    "A page parent must be in the same project, branch, and language".to_string(),
                ));
            }
            parents.insert(id.clone(), parent_id.clone());
            positions.insert(id.clone(), *position);
        }

        fn resolve_path(
            id: &str,
            nodes: &std::collections::HashMap<String, PageReorderRow>,
            parents: &std::collections::HashMap<String, Option<String>>,
            resolved: &mut std::collections::HashMap<String, String>,
            visiting: &mut std::collections::HashSet<String>,
            depth: usize,
        ) -> Result<String, AppError> {
            if let Some(path) = resolved.get(id) {
                return Ok(path.clone());
            }
            if depth >= 256 {
                return Err(AppError::Validation(
                    "Page trees cannot exceed 256 levels".to_string(),
                ));
            }
            if !visiting.insert(id.to_string()) {
                return Err(AppError::Conflict(
                    "The proposed page tree contains a parent cycle".to_string(),
                ));
            }
            let node = nodes
                .get(id)
                .ok_or_else(|| AppError::Conflict("A page parent is outside the tree".to_string()))?;
            if node.slug.is_empty()
                || node.slug == "."
                || node.slug == ".."
                || node.slug.contains('/')
                || node.slug.contains('\\')
            {
                return Err(AppError::Validation(
                    "Page tree contains an invalid slug".to_string(),
                ));
            }
            let path = match parents.get(id).and_then(Option::as_deref) {
                Some(parent_id) => format!(
                    "{}/{}",
                    resolve_path(parent_id, nodes, parents, resolved, visiting, depth + 1)?
                        .trim_end_matches('/'),
                    node.slug
                ),
                None => format!("/{}", node.slug),
            };
            visiting.remove(id);
            if path.len() > 1024 {
                return Err(AppError::Validation(
                    "A page path would exceed 1024 bytes".to_string(),
                ));
            }
            resolved.insert(id.to_string(), path.clone());
            Ok(path)
        }

        let mut resolved = std::collections::HashMap::new();
        let mut visiting = std::collections::HashSet::new();
        for id in nodes.keys() {
            resolve_path(id, &nodes, &parents, &mut resolved, &mut visiting, 0)?;
        }
        let mut unique_paths = std::collections::HashMap::new();
        for (id, path) in &resolved {
            if let Some(existing) = unique_paths.insert(path, id) {
                return Err(AppError::Conflict(format!(
                    "Pages {existing} and {id} would have the same path"
                )));
            }
        }

        let token = Uuid::new_v4().simple().to_string();
        sqlx::query(
            r#"UPDATE "Page"
               SET path = '/_cms_reorder_' || $1 || '/' || id
               WHERE project_id = $2 AND branch_id = $3
                 AND language_id IS NOT DISTINCT FROM $4"#,
        )
        .bind(&token)
        .bind(project_id)
        .bind(&branch_id)
        .bind(language_id.as_deref())
        .execute(&mut *tx)
        .await
        .map_err(map_page_write_error)?;

        let mut page_ids = Vec::with_capacity(nodes.len());
        let mut paths = Vec::with_capacity(nodes.len());
        let mut parent_ids = Vec::with_capacity(nodes.len());
        let mut page_positions = Vec::with_capacity(nodes.len());
        for id in nodes.keys() {
            page_ids.push(id.clone());
            paths.push(resolved[id].clone());
            parent_ids.push(parents[id].clone());
            page_positions.push(positions[id]);
        }
        sqlx::query(
            r#"
            UPDATE "Page" AS page
            SET path = proposed.path,
                parent_id = proposed.parent_id,
                position = proposed.position,
                updated_at = $5
            FROM UNNEST($1::TEXT[], $2::TEXT[], $3::TEXT[], $4::INTEGER[])
                 AS proposed(id, path, parent_id, position)
            WHERE page.id = proposed.id
              AND page.project_id = $6 AND page.branch_id = $7
              AND page.language_id IS NOT DISTINCT FROM $8
            "#,
        )
        .bind(&page_ids)
        .bind(&paths)
        .bind(&parent_ids)
        .bind(&page_positions)
        .bind(Utc::now())
        .bind(project_id)
        .bind(&branch_id)
        .bind(language_id.as_deref())
        .execute(&mut *tx)
        .await
        .map_err(map_page_write_error)?;

        let reordered = sqlx::query_as::<_, PageRow>(
            "SELECT * FROM \"Page\" WHERE id = ANY($1) ORDER BY parent_id NULLS FIRST, position, id",
        )
        .bind(&requested_ids)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(reordered.into_iter().map(Into::into).collect())
    }

    /// Reorder pages (update positions atomically)
    pub async fn reorder(pool: &PgPool, page_ids: &[String]) -> Result<Vec<Page>, AppError> {
        let mut transaction = pool
            .begin()
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        for (index, page_id) in page_ids.iter().enumerate() {
            let position = index as i32;

            sqlx::query("UPDATE \"Page\" SET position = $1, updated_at = $2 WHERE id = $3")
                .bind(position)
                .bind(Utc::now())
                .bind(page_id)
                .execute(&mut *transaction)
                .await
                .map_err(|e| AppError::Database(e.into()))?;
        }

        transaction
            .commit()
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        // Return the reordered pages
        let pages = sqlx::query_as::<_, PageRow>(
            "SELECT * FROM \"Page\" WHERE id = ANY($1) ORDER BY position ASC",
        )
        .bind(page_ids)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(pages.into_iter().map(|r| r.into()).collect())
    }

    /// Check if a path is available in a branch and language.
    /// Since the unique constraint is on (project_id, branch_id, language_id, path),
    /// we check path uniqueness — not raw slug — to avoid false conflicts between
    /// pages under different parents that share the same slug.
    pub async fn is_slug_available(
        pool: &PgPool,
        project_id: &str,
        branch_id: &str,
        language_id: Option<&str>,
        slug: &str,
        exclude_page_id: Option<&str>,
    ) -> Result<bool, AppError> {
        // Build the candidate path prefix to check — we can't know the full path
        // without the parent, so we check all paths that END with /<slug> at root
        // OR that exactly equal /<slug>. For the full path check, callers should
        // use is_path_available instead.  This function remains for root-level checks.
        Self::is_path_available(
            pool,
            project_id,
            branch_id,
            language_id,
            &format!("/{}", slug),
            exclude_page_id,
        )
        .await
    }

    /// Check if a full path is available in a branch and language.
    pub async fn is_path_available(
        pool: &PgPool,
        project_id: &str,
        branch_id: &str,
        language_id: Option<&str>,
        path: &str,
        exclude_page_id: Option<&str>,
    ) -> Result<bool, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM \"Page\" WHERE project_id = ");
        query_builder.push_bind(project_id);
        query_builder.push(" AND branch_id = ");
        query_builder.push_bind(branch_id);

        if let Some(lang_id) = language_id {
            if !lang_id.is_empty() {
                query_builder.push(" AND language_id = ");
                query_builder.push_bind(lang_id);
            } else {
                query_builder.push(" AND language_id IS NULL");
            }
        } else {
            query_builder.push(" AND language_id IS NULL");
        }

        query_builder.push(" AND path = ");
        query_builder.push_bind(path);

        if let Some(exclude_id) = exclude_page_id {
            query_builder.push(" AND id != ");
            query_builder.push_bind(exclude_id);
        }

        let count: i64 = query_builder
            .build()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?
            .get::<i64, _>(0);

        Ok(count == 0)
    }

    /// Detect if moving a page would create a cycle in the tree
    pub async fn would_create_cycle(
        pool: &PgPool,
        page_id: &str,
        new_parent_id: Option<&str>,
    ) -> Result<bool, AppError> {
        if new_parent_id.is_none() {
            // Moving to root can't create a cycle
            return Ok(false);
        }

        let new_parent_id = new_parent_id.unwrap();

        // Check if new_parent would be a descendant of page_id
        // We do this by checking if any ancestor of new_parent is page_id
        let mut current_id = new_parent_id.to_string();
        let mut visited = std::collections::HashSet::new();

        loop {
            if current_id == page_id {
                return Ok(true); // Would create cycle
            }

            if visited.contains(&current_id) {
                break; // Prevent infinite loops
            }
            visited.insert(current_id.clone());

            let parent_id: Option<String> =
                sqlx::query_scalar("SELECT parent_id FROM \"Page\" WHERE id = $1")
                    .bind(&current_id)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| AppError::Database(e.into()))?;

            if let Some(parent) = parent_id {
                current_id = parent;
            } else {
                break;
            }
        }

        Ok(false)
    }

    /// Get the maximum sort position among sibling pages
    pub async fn get_max_position(
        pool: &PgPool,
        project_id: &str,
        branch_id: &str,
        language_id: Option<&str>,
        parent_id: Option<&str>,
    ) -> Result<i32, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT MAX(position) FROM \"Page\" WHERE project_id = ");
        query_builder.push_bind(project_id);
        query_builder.push(" AND branch_id = ");
        query_builder.push_bind(branch_id);

        if let Some(lang_id) = language_id {
            if !lang_id.is_empty() {
                query_builder.push(" AND language_id = ");
                query_builder.push_bind(lang_id);
            }
        }

        if let Some(pid) = parent_id {
            if pid.is_empty() {
                query_builder.push(" AND parent_id IS NULL");
            } else {
                query_builder.push(" AND parent_id = ");
                query_builder.push_bind(pid);
            }
        } else {
            query_builder.push(" AND parent_id IS NULL");
        }

        let count: Option<i32> = query_builder
            .build_query_scalar()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(count.unwrap_or(0))
    }

    /// Resolve search result IDs against the current publish/index state and
    /// requested branch/language scope. Search-engine metadata is not trusted as
    /// an authorization or tenancy boundary.
    pub async fn get_searchable_by_ids_in_scope(
        pool: &PgPool,
        page_ids: &[&str],
        project_id: &str,
        branch_id: Option<&str>,
        language_id: Option<&str>,
    ) -> Result<Vec<Page>, AppError> {
        if page_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<_, PageRow>(
            r#"SELECT * FROM "Page"
               WHERE id = ANY($1)
                 AND project_id = $2
                 AND ($3::TEXT IS NULL OR branch_id = $3)
                 AND ($4::TEXT IS NULL OR language_id = $4)
                 AND is_published = TRUE
                 AND is_indexed = TRUE
                 AND UPPER(kind) = 'PAGE'
               ORDER BY position ASC, path ASC"#,
        )
        .bind(page_ids)
        .bind(project_id)
        .bind(branch_id)
        .bind(language_id)
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Get multiple pages by IDs
    pub async fn get_by_ids(pool: &PgPool, page_ids: &[&str]) -> Result<Vec<Page>, AppError> {
        if page_ids.is_empty() {
            return Ok(vec![]);
        }
        let rows = sqlx::query_as::<_, PageRow>(
            "SELECT * FROM \"Page\" WHERE id = ANY($1) ORDER BY position ASC",
        )
        .bind(page_ids)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Update a page's path (slug/breadcrumb)
    pub async fn update_path(pool: &PgPool, page_id: &str, path: &str) -> Result<Page, AppError> {
        let row = sqlx::query_as::<_, PageRow>(
            "UPDATE \"Page\" SET path = $1, updated_at = $2 WHERE id = $3 RETURNING *",
        )
        .bind(path)
        .bind(Utc::now())
        .bind(page_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;
        Ok(row.into())
    }

    /// Get child pages of a parent
    pub async fn get_by_parent(
        pool: &PgPool,
        project_id: &str,
        branch_id: &str,
        parent_id: &str,
    ) -> Result<Vec<Page>, AppError> {
        let rows = sqlx::query_as::<_, PageRow>(
            "SELECT * FROM \"Page\" WHERE project_id = $1 AND branch_id = $2 AND parent_id = $3 \
             ORDER BY position ASC",
        )
        .bind(project_id)
        .bind(branch_id)
        .bind(parent_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Count pages in a branch
    pub async fn count_by_branch(pool: &PgPool, branch_id: &str) -> Result<i64, AppError> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM \"Page\" WHERE branch_id = $1")
            .bind(branch_id)
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;
        Ok(row.get::<i64, _>("count"))
    }

    /// Get all pages in a project (for reindexing)
    pub async fn get_by_project(pool: &PgPool, project_id: &str) -> Result<Vec<Page>, AppError> {
        let rows = sqlx::query_as::<_, PageRow>(
            "SELECT * FROM \"Page\" WHERE project_id = $1 ORDER BY created_at ASC",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

fn push_page_update_separator(query: &mut QueryBuilder<Postgres>, has_updates: &mut bool) {
    if *has_updates {
        query.push(", ");
    }
    *has_updates = true;
}

fn map_page_write_error(error: sqlx::Error) -> AppError {
    match &error {
        sqlx::Error::Database(db_error) if db_error.code().as_deref() == Some("23505") => {
            AppError::Conflict("Page path already exists in this branch and language".to_string())
        }
        _ => AppError::Database(error),
    }
}
