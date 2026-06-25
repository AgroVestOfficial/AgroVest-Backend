use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

impl PaginationParams {
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn per_page(&self) -> u32 {
        self.per_page.unwrap_or(20).clamp(1, 100)
    }

    pub fn offset(&self) -> u32 {
        (self.page() - 1) * self.per_page()
    }
}

#[derive(Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

// Manually implement Deserialize for PaginatedResponse<T> where T is DeserializeOwned
impl<'de, T> Deserialize<'de> for PaginatedResponse<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper<T> {
            data: Vec<T>,
            total: i64,
            page: u32,
            per_page: u32,
            total_pages: u32,
        }

        let helper = Helper::deserialize(deserializer)?;
        Ok(PaginatedResponse {
            data: helper.data,
            total: helper.total,
            page: helper.page,
            per_page: helper.per_page,
            total_pages: helper.total_pages,
        })
    }
}

impl<T: Serialize> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: i64, page: u32, per_page: u32) -> Self {
        let total_pages = ((total as f64) / (per_page as f64)).ceil() as u32;
        Self {
            data,
            total,
            page,
            per_page,
            total_pages,
        }
    }
}
