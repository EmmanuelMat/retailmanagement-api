//! Shared pagination/sort primitives for `/v1/*` list endpoints.
//! Query params are camelCase (matches the existing ListProductosParams
//! convention); response envelope fields are snake_case (matches the
//! existing struct Serialize output across the codebase).

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct PageParams {
    pub page: Option<i64>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<i64>,
}

impl PageParams {
    pub fn page_number(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    /// `default_size` lets each endpoint pick its own default (e.g. Auditoria wants 10).
    /// Clamped to 5000 rather than a tighter UI-sized limit because a few
    /// callers (POS product picker, purchase/quote item pickers) intentionally
    /// request the full catalog in one page instead of using true pagination.
    pub fn limit(&self, default_size: i64) -> i64 {
        self.page_size.unwrap_or(default_size).clamp(1, 5000)
    }

    pub fn offset(&self, default_size: i64) -> i64 {
        (self.page_number() - 1) * self.limit(default_size)
    }
}

#[derive(Debug, Deserialize)]
pub struct SortParams {
    #[serde(rename = "sortBy")]
    pub sort_by: Option<String>,
    #[serde(rename = "sortDir")]
    pub sort_dir: Option<String>,
}

impl SortParams {
    /// Resolves against a fixed allow-list of (query_value, sql_column) pairs.
    /// The returned string is safe to interpolate directly into `ORDER BY`
    /// because it can only ever be one of the hardcoded `allowed` values,
    /// never raw user input - an unrecognized `sortBy` silently falls back
    /// to `default` instead of erroring or reflecting into SQL.
    pub fn resolve(&self, allowed: &[(&str, &str)], default: &str) -> String {
        let dir = match self.sort_dir.as_deref() {
            Some("asc") => "ASC",
            _ => "DESC",
        };
        match self
            .sort_by
            .as_deref()
            .and_then(|s| allowed.iter().find(|(k, _)| *k == s))
        {
            Some((_, col)) => format!("{col} {dir}"),
            None => default.to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Page<T: Serialize> {
    pub items: Vec<T>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
    pub total_pages: i64,
}

impl<T: Serialize> Page<T> {
    pub fn new(items: Vec<T>, page: i64, page_size: i64, total: i64) -> Self {
        let total_pages = ((total as f64) / (page_size as f64)).ceil().max(1.0) as i64;
        Self {
            items,
            page,
            page_size,
            total,
            total_pages,
        }
    }
}
