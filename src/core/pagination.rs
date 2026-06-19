use std::future::Future;

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::core::error::AppResult;

/// Default page size
pub const DEFAULT_PAGE_SIZE: u32 = 20;

/// Maximum page size
pub const MAX_PAGE_SIZE: u32 = 100;

/// Pagination query parameters
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct PaginationParams {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    pub page: u32,

    /// Number of items per page
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    DEFAULT_PAGE_SIZE
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: DEFAULT_PAGE_SIZE,
        }
    }
}

impl PaginationParams {
    /// Calculate the offset for SQL queries (using the capped page size)
    pub fn offset(&self) -> u32 {
        (self.page.saturating_sub(1)) * self.limit()
    }

    /// Get the limit, capped at MAX_PAGE_SIZE
    pub fn limit(&self) -> u32 {
        self.page_size.min(MAX_PAGE_SIZE)
    }
}

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationMeta,
}

/// Pagination metadata
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaginationMeta {
    pub page: u32,
    pub page_size: u32,
    pub total_items: u64,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_prev: bool,
}

impl PaginationMeta {
    pub fn new(page: u32, page_size: u32, total_items: u64) -> Self {
        let total_pages = ((total_items as f64) / (page_size as f64)).ceil() as u32;
        Self {
            page,
            page_size,
            total_items,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        }
    }
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, page: u32, page_size: u32, total_items: u64) -> Self {
        Self {
            data,
            pagination: PaginationMeta::new(page, page_size, total_items),
        }
    }
}

/// Run a paginated query: fetch one capped page plus its total, and assemble
/// the response. `items` receives the capped `limit` and `offset` (both `i64`,
/// ready to bind); `count` returns the unfiltered total (use `COUNT(*) as
/// "count!"` so it is non-null `i64`). Both closures return raw `sqlx` results.
pub async fn paginate<T, IFut, CFut>(
    params: &PaginationParams,
    items: impl FnOnce(i64, i64) -> IFut,
    count: impl FnOnce() -> CFut,
) -> AppResult<PaginatedResponse<T>>
where
    IFut: Future<Output = Result<Vec<T>, sqlx::Error>>,
    CFut: Future<Output = Result<i64, sqlx::Error>>,
{
    let limit = params.limit();
    let data = items(limit as i64, params.offset() as i64).await?;
    let total = count().await?;
    Ok(PaginatedResponse::new(
        data,
        params.page,
        limit,
        total as u64,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_offset() {
        let params = PaginationParams {
            page: 1,
            page_size: 20,
        };
        assert_eq!(params.offset(), 0);

        let params = PaginationParams {
            page: 2,
            page_size: 20,
        };
        assert_eq!(params.offset(), 20);

        let params = PaginationParams {
            page: 3,
            page_size: 10,
        };
        assert_eq!(params.offset(), 20);
    }

    #[test]
    fn test_pagination_limit_capped() {
        let params = PaginationParams {
            page: 1,
            page_size: 200,
        };
        assert_eq!(params.limit(), MAX_PAGE_SIZE);
    }

    #[tokio::test]
    async fn test_paginate_caps_limit_and_assembles() {
        // page_size over the cap must reach the items closure as MAX_PAGE_SIZE,
        // with the offset derived from the capped limit.
        let params = PaginationParams {
            page: 3,
            page_size: 200,
        };

        let page: PaginatedResponse<u8> = paginate(
            &params,
            |limit, offset| async move {
                assert_eq!(limit, MAX_PAGE_SIZE as i64);
                assert_eq!(offset, (MAX_PAGE_SIZE as i64) * 2);
                Ok(vec![1u8, 2, 3])
            },
            || async { Ok(530) },
        )
        .await
        .expect("paginate succeeds");

        assert_eq!(page.data, vec![1, 2, 3]);
        assert_eq!(page.pagination.page, 3);
        assert_eq!(page.pagination.page_size, MAX_PAGE_SIZE);
        assert_eq!(page.pagination.total_items, 530);
    }

    #[test]
    fn test_pagination_meta() {
        let meta = PaginationMeta::new(1, 20, 100);
        assert_eq!(meta.total_pages, 5);
        assert!(meta.has_next);
        assert!(!meta.has_prev);

        let meta = PaginationMeta::new(5, 20, 100);
        assert!(!meta.has_next);
        assert!(meta.has_prev);
    }
}
