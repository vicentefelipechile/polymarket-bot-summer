//! Market analyzer using AI for intelligent trading recommendations

use crate::ai::client::{AiResponse, GeminiClient};
use crate::ai::personality::{AiPersonality, PersonalityTrait};
use crate::data::DbPool;
use crate::trading::markets::MarketInfo;
use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::Row;

/// Recommended trading action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TradeAction {
    Buy(String),  // Buy with outcome name
    Sell(String), // Sell with outcome name
    Hold,
}

impl std::fmt::Display for TradeAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Buy(outcome) => write!(f, "Buy {}", outcome),
            Self::Sell(outcome) => write!(f, "Sell {}", outcome),
            Self::Hold => write!(f, "Hold"),
        }
    }
}

/// AI recommendation for a market
#[derive(Debug, Clone)]
pub struct AiRecommendation {
    pub market_id: String,
    pub action: TradeAction,
    pub confidence: f64, // 0.0 - 1.0
    pub reasoning: String,
    pub analysis: String,
    pub timestamp: i64,
}

/// Market analyzer using AI
pub struct MarketAnalyzer {
    client: GeminiClient,
    personality: AiPersonality,
    db: DbPool,
}

impl MarketAnalyzer {
    /// Create a new market analyzer
    pub fn new(client: GeminiClient, personality: AiPersonality, db: DbPool) -> Self {
        Self {
            client,
            personality,
            db,
        }
    }

    /// Analyze a specific market and generate recommendation
    pub async fn analyze_market(&self, market: &MarketInfo) -> Result<AiRecommendation> {
        let prompt = self.build_analysis_prompt(market);
        let system_instruction = self.personality.system_prompt();

        let response = self
            .client
            .generate_with_retry(&prompt, Some(system_instruction), 2)
            .await
            .context("Failed to get AI analysis")?;

        let recommendation = self.parse_ai_response(&response, market)?;

        // Save to database
        self.save_recommendation(&recommendation).await?;

        Ok(recommendation)
    }

    /// Analyze multiple markets and return recommendations
    pub async fn analyze_markets(&self, markets: &[MarketInfo]) -> Result<Vec<AiRecommendation>> {
        let mut recommendations = Vec::new();

        for market in markets {
            match self.analyze_market(market).await {
                Ok(rec) => recommendations.push(rec),
                Err(e) => {
                    tracing::warn!("Failed to analyze market {}: {}", market.id, e);
                }
            }
        }

        Ok(recommendations)
    }

    /// Build analysis prompt for AI
    fn build_analysis_prompt(&self, market: &MarketInfo) -> String {
        let prices_str = market
            .prices
            .iter()
            .enumerate()
            .map(|(i, price)| {
                let outcome = market
                    .outcomes
                    .get(i)
                    .map(|s| s.as_str())
                    .unwrap_or("Unknown");
                format!("  - {}: {:.2}%", outcome, price * 100.0)
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"Analiza este mercado de predicción y proporciona una recomendación de trading:

MERCADO: {}
ID: {}
ACTIVO: {}
VOLUMEN: {}

RESULTADOS Y PRECIOS:
{}

TAREA:
1. Analiza la pregunta y las probabilidades actuales
2. Evalúa si hay una oportunidad de trading basada en:
   - Precios actuales vs probabilidad real estimada
   - Volumen y liquidez del mercado
   - Claridad de la pregunta y resultado
3. Proporciona una recomendación objetiva

FORMATO DE RESPUESTA (JSON):
{{
  "action": "buy" | "sell" | "hold",
  "outcome": "nombre del outcome si buy/sell, null si hold",
  "confidence": 0.0-1.0,
  "reasoning": "explicación breve (1-2 líneas)"
}}

IMPORTANTE: 
- Solo recomienda "buy" o "sell" si hay una clara oportunidad basada en datos
- La mayoría de los mercados deberían ser "hold" a menos que haya un desequilibrio evidente
- Tu análisis debe ser puramente objetivo, sin importar tu personalidad"#,
            market.question,
            market.id,
            if market.active { "Sí" } else { "No" },
            market.volume,
            prices_str
        )
    }

    /// Parse AI response into recommendation
    fn parse_ai_response(
        &self,
        response: &AiResponse,
        market: &MarketInfo,
    ) -> Result<AiRecommendation> {
        // Try to extract JSON from response
        let json_start = response.text.find('{');
        let json_end = response.text.rfind('}');

        let json_str = if let (Some(start), Some(end)) = (json_start, json_end) {
            &response.text[start..=end]
        } else {
            &response.text
        };

        #[derive(Deserialize)]
        struct AiResponseParsed {
            action: String,
            outcome: Option<String>,
            confidence: f64,
            reasoning: String,
        }

        let parsed: AiResponseParsed =
            serde_json::from_str(json_str).context("Failed to parse AI response as JSON")?;

        let action = match parsed.action.to_lowercase().as_str() {
            "buy" => {
                let outcome = parsed.outcome.context("Buy action requires outcome")?;
                TradeAction::Buy(outcome)
            }
            "sell" => {
                let outcome = parsed.outcome.context("Sell action requires outcome")?;
                TradeAction::Sell(outcome)
            }
            "hold" => TradeAction::Hold,
            other => anyhow::bail!("Invalid action: {}", other),
        };

        // Clamp confidence to valid range
        let confidence = parsed.confidence.clamp(0.0, 1.0);

        Ok(AiRecommendation {
            market_id: market.id.clone(),
            action,
            confidence,
            reasoning: parsed.reasoning,
            analysis: self.personality.format_message(&response.text),
            timestamp: Utc::now().timestamp(),
        })
    }

    /// Save recommendation to database
    async fn save_recommendation(&self, rec: &AiRecommendation) -> Result<()> {
        let action_str = match &rec.action {
            TradeAction::Buy(outcome) => format!("buy:{}", outcome),
            TradeAction::Sell(outcome) => format!("sell:{}", outcome),
            TradeAction::Hold => "hold".to_string(),
        };

        sqlx::query(
            r#"
            INSERT INTO ai_recommendations 
            (market_id, action, confidence, reasoning, analysis, timestamp, personality)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&rec.market_id)
        .bind(&action_str)
        .bind(rec.confidence)
        .bind(&rec.reasoning)
        .bind(&rec.analysis)
        .bind(rec.timestamp)
        .bind(self.personality.to_string())
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get recent recommendations from database
    pub async fn get_recent_recommendations(&self, limit: usize) -> Result<Vec<AiRecommendation>> {
        let rows = sqlx::query(
            r#"
            SELECT market_id, action, confidence, reasoning, analysis, timestamp
            FROM ai_recommendations
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.db)
        .await?;

        let mut recommendations = Vec::new();
        for row in rows {
            let market_id: String = row.get(0);
            let action_str: String = row.get(1);
            let confidence: f64 = row.get(2);
            let reasoning: String = row.get(3);
            let analysis: String = row.get(4);
            let timestamp: i64 = row.get(5);

            let action = if action_str == "hold" {
                TradeAction::Hold
            } else if let Some(outcome) = action_str.strip_prefix("buy:") {
                TradeAction::Buy(outcome.to_string())
            } else if let Some(outcome) = action_str.strip_prefix("sell:") {
                TradeAction::Sell(outcome.to_string())
            } else {
                continue; // Skip invalid entries
            };

            recommendations.push(AiRecommendation {
                market_id,
                action,
                confidence,
                reasoning,
                analysis,
                timestamp,
            });
        }

        Ok(recommendations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trade_action_display() {
        assert_eq!(TradeAction::Hold.to_string(), "Hold");
        assert_eq!(TradeAction::Buy("Yes".to_string()).to_string(), "Buy Yes");
        assert_eq!(TradeAction::Sell("No".to_string()).to_string(), "Sell No");
    }

    #[test]
    fn test_prompt_building() {
        let market = MarketInfo {
            id: "test_id".to_string(),
            question: "Will it rain tomorrow?".to_string(),
            active: true,
            order_book_enabled: true,
            volume: "1000".to_string(),
            outcomes: vec!["Yes".to_string(), "No".to_string()],
            prices: vec![0.6, 0.4],
        };

        let analyzer = MarketAnalyzer {
            client: GeminiClient::new("test_key".to_string()),
            personality: AiPersonality::Summer,
            db: unimplemented!(),
        };

        let prompt = analyzer.build_analysis_prompt(&market);

        // Test that prompt contains expected market data
        assert!(prompt.contains("Will it rain tomorrow?"));
        assert!(prompt.contains("Yes: 60.00%"));
        assert!(prompt.contains("No: 40.00%"));
        assert!(prompt.contains(&market.id));
        assert!(prompt.contains(&market.volume));
    }
}
