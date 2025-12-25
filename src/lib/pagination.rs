use serde::{Deserialize, Serialize};

/// Default page size
pub const DEFAULT_PAGE_SIZE: u32 = 20;

/// Maximum page size
pub const MAX_PAGE_SIZE: u32 = 100;

/// Pagination query parameters
#[derive(Debug, Clone, Deserialize)]
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
    /// Calculate the offset for SQL queries
    pub fn offset(&self) -> u32 {
        (self.page.saturating_sub(1)) * self.page_size
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
#[derive(Debug, Clone, Serialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_offset() {
        let params = PaginationParams { page: 1, page_size: 20 };
        assert_eq!(params.offset(), 0);

        let params = PaginationParams { page: 2, page_size: 20 };
        assert_eq!(params.offset(), 20);

        let params = PaginationParams { page: 3, page_size: 10 };
        assert_eq!(params.offset(), 20);
    }

    #[test]
    fn test_pagination_limit_capped() {
        let params = PaginationParams { page: 1, page_size: 200 };
        assert_eq!(params.limit(), MAX_PAGE_SIZE);
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
