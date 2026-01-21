use anyhow::Result;
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::Row; // For .get() method on database rows

const GAMMA_API_BASE: &str = "https://gamma-api.polymarket.com";

/// Custom deserializer that handles both JSON arrays and JSON strings containing arrays
fn deserialize_string_or_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    use serde_json::Value;

    let value = Value::deserialize(deserializer)?;

    match value {
        Value::Array(arr) => {
            // Direct array - convert each element to string
            arr.into_iter()
                .map(|v| match v {
                    Value::String(s) => Ok(s),
                    other => Ok(other.to_string().trim_matches('"').to_string()),
                })
                .collect()
        }
        Value::String(s) => {
            // String that might contain a JSON array
            if s.starts_with('[') {
                serde_json::from_str(&s).map_err(|e| D::Error::custom(e.to_string()))
            } else if s.is_empty() {
                Ok(Vec::new())
            } else {
                // Single value
                Ok(vec![s])
            }
        }
        Value::Null => Ok(Vec::new()),
        other => Err(D::Error::custom(format!(
            "expected array or string, found: {}",
            other
        ))),
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

/// Helper structs for /public-search response
#[derive(Debug, Deserialize)]
struct PublicSearchResponse {
    #[serde(default)]
    events: Vec<PublicSearchEvent>,
    #[serde(default)]
    tags: serde_json::Value,
    #[serde(default)]
    profiles: serde_json::Value,
    #[serde(default)]
    pagination: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct PublicSearchEvent {
    #[serde(default)]
    markets: Vec<PublicSearchMarket>,
}

/// Market data from /public-search endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicSearchMarket {
    pub id: String,
    pub question: String,
    #[serde(default)]
    pub volume: Option<String>,
    #[serde(default)]
    pub closed: bool,
    #[serde(rename = "enableOrderBook", default)]
    pub enable_order_book: bool,
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

impl From<PublicSearchMarket> for MarketInfo {
    fn from(m: PublicSearchMarket) -> Self {
        Self {
            id: m.id,
            question: m.question,
            active: !m.closed,
            order_book_enabled: m.enable_order_book,
            volume: m.volume.unwrap_or_else(|| "0".to_string()),
            outcomes: Vec::new(), // public-search doesn't provide outcomes
            prices: Vec::new(),   // public-search doesn't provide prices
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

    /// Search markets by keyword using /public-search
    pub async fn search_markets(&self, keyword: &str, _limit: usize) -> Result<Vec<MarketInfo>> {
        let url = format!(
            "{}/public-search?q={}&search_profiles=false",
            GAMMA_API_BASE, keyword
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to fetch markets: {}", response.status());
        }

        let search_response: PublicSearchResponse = response.json().await?;

        // Flatten events -> markets
        let markets: Vec<PublicSearchMarket> = search_response
            .events
            .into_iter()
            .flat_map(|e| e.markets)
            .collect();

        // Filter valid CLOB markets that are open and convert
        let filtered: Vec<MarketInfo> = markets
            .into_iter()
            .filter(|m| m.enable_order_book && !m.closed)
            .map(|m| m.into())
            .take(20)
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

// ============================================================================
// Database Persistence Functions
// ============================================================================

use crate::database::DbPool;
use chrono::Utc;

/// Save a watched market to the database
pub async fn save_watched_market(pool: &DbPool, market: &MarketInfo) -> Result<()> {
    let outcomes_json = serde_json::to_string(&market.outcomes)?;
    let prices_json = serde_json::to_string(&market.prices)?;
    let now = Utc::now().timestamp();

    sqlx::query(
        r#"
        INSERT OR REPLACE INTO watched_markets 
        (id, question, volume, outcomes, prices, joined_at, active)
        VALUES (?, ?, ?, ?, ?, ?, 1)
        "#,
    )
    .bind(&market.id)
    .bind(&market.question)
    .bind(&market.volume)
    .bind(outcomes_json)
    .bind(prices_json)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

/// Load all active watched markets from the database
pub async fn load_watched_markets(pool: &DbPool) -> Result<Vec<MarketInfo>> {
    let rows = sqlx::query(
        r#"
        SELECT id, question, volume, outcomes, prices
        FROM watched_markets
        WHERE active = 1
        ORDER BY joined_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut markets = Vec::new();
    for row in rows {
        let id: String = row.get(0);
        let question: String = row.get(1);
        let volume: String = row.get(2);
        let outcomes_json: String = row.get(3);
        let prices_json: String = row.get(4);

        let outcomes: Vec<String> = serde_json::from_str(&outcomes_json).unwrap_or_default();
        let prices: Vec<f64> = serde_json::from_str(&prices_json).unwrap_or_default();

        markets.push(MarketInfo {
            id,
            question,
            active: true,
            order_book_enabled: true,
            volume,
            outcomes,
            prices,
        });
    }

    Ok(markets)
}

/// Remove a watched market from the database (mark as inactive)
pub async fn remove_watched_market(pool: &DbPool, id: &str) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE watched_markets 
        SET active = 0
        WHERE id = ?
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}
