use anyhow::Result;
use serde::{Deserialize, Deserializer, Serialize};

const GAMMA_API_BASE: &str = "https://gamma-api.polymarket.com";

/// Custom deserializer that handles both JSON arrays and JSON strings containing arrays
fn deserialize_string_or_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        String(String),
        Vec(Vec<String>),
    }

    match StringOrVec::deserialize(deserializer)? {
        StringOrVec::String(s) => {
            // Try to parse as JSON array
            if s.starts_with('[') {
                serde_json::from_str(&s).map_err(|e| D::Error::custom(e.to_string()))
            } else if s.is_empty() {
                Ok(Vec::new())
            } else {
                // Single value
                Ok(vec![s])
            }
        }
        StringOrVec::Vec(v) => Ok(v),
    }
}

/// Market data from Polymarket Gamma API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GammaMarket {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub question: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub closed: bool,
    #[serde(rename = "enableOrderBook", default)]
    pub enable_order_book: bool,
    #[serde(rename = "conditionId", default)]
    pub condition_id: String,
    #[serde(rename = "questionId", default)]
    pub question_id: String,
    #[serde(default)]
    pub volume: String,
    #[serde(default)]
    pub liquidity: String,
    #[serde(rename = "startDate", default)]
    pub start_date: Option<String>,
    #[serde(rename = "endDate", default)]
    pub end_date: Option<String>,
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub outcomes: Vec<String>,
    #[serde(
        rename = "outcomePrices",
        default,
        deserialize_with = "deserialize_string_or_vec"
    )]
    pub outcome_prices: Vec<String>,
}

/// Simplified market info for display
#[derive(Debug, Clone)]
pub struct MarketInfo {
    pub id: String,
    pub question: String,
    pub active: bool,
    pub order_book_enabled: bool,
    pub volume: String,
    pub outcomes: Vec<String>,
    pub prices: Vec<f64>,
}

impl From<GammaMarket> for MarketInfo {
    fn from(m: GammaMarket) -> Self {
        let prices: Vec<f64> = m
            .outcome_prices
            .iter()
            .filter_map(|p| p.parse::<f64>().ok())
            .collect();

        // Use id if condition_id is empty
        let id = if m.condition_id.is_empty() {
            m.id.clone()
        } else {
            m.condition_id.clone()
        };

        Self {
            id,
            question: m.question,
            active: m.active && !m.closed,
            order_book_enabled: m.enable_order_book,
            volume: m.volume,
            outcomes: m.outcomes,
            prices,
        }
    }
}

/// Market service for fetching markets from Polymarket
pub struct MarketService {
    client: reqwest::Client,
}

impl MarketService {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Search markets by keyword
    pub async fn search_markets(&self, keyword: &str, limit: usize) -> Result<Vec<MarketInfo>> {
        let url = format!(
            "{}/markets?limit={}&closed=false&active=true",
            GAMMA_API_BASE, limit
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to fetch markets: {}", response.status());
        }

        let markets: Vec<GammaMarket> = response.json().await?;

        // Filter by keyword (case-insensitive)
        let keyword_lower = keyword.to_lowercase();
        let filtered: Vec<MarketInfo> = markets
            .into_iter()
            .filter(|m| {
                m.question.to_lowercase().contains(&keyword_lower)
                    || m.description.to_lowercase().contains(&keyword_lower)
            })
            .filter(|m| m.enable_order_book) // Only CLOB-enabled markets
            .map(|m| m.into())
            .take(20) // Limit results for display
            .collect();

        Ok(filtered)
    }

    /// Fetch featured/trending markets
    pub async fn get_trending_markets(&self, limit: usize) -> Result<Vec<MarketInfo>> {
        let url = format!(
            "{}/markets?limit={}&closed=false&active=true&order=volume&ascending=false",
            GAMMA_API_BASE, limit
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to fetch markets: {}", response.status());
        }

        let markets: Vec<GammaMarket> = response.json().await?;

        let filtered: Vec<MarketInfo> = markets
            .into_iter()
            .filter(|m| m.enable_order_book)
            .map(|m| m.into())
            .collect();

        Ok(filtered)
    }

    /// Get market by ID
    pub async fn get_market(&self, condition_id: &str) -> Result<Option<MarketInfo>> {
        let url = format!("{}/markets?id={}", GAMMA_API_BASE, condition_id);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let markets: Vec<GammaMarket> = response.json().await?;
        Ok(markets.into_iter().next().map(|m| m.into()))
    }
}

impl Default for MarketService {
    fn default() -> Self {
        Self::new()
    }
}
