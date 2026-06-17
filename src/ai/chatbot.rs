//! AI chatbot with Gemini function-calling for conversational market actions.

// =========================================================================================================
// Imports
// =========================================================================================================

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::ai::{AiPersonality, GeminiClient, PersonalityTrait};
use crate::data::DbPool;
use crate::trading::ExecutionEngine;

// =========================================================================================================
// Gemini API Function Calling Types (Official Format)
// =========================================================================================================

/// Function declaration in Gemini API format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: JsonSchema,
}

/// JSON Schema for function parameters (OpenAPI subset)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchema {
    #[serde(rename = "type")]
    pub schema_type: String, // "object"
    pub properties: HashMap<String, PropertySchema>,
    pub required: Vec<String>,
}

/// Property definition in JSON Schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySchema {
    #[serde(rename = "type")]
    pub property_type: String, // "string", "integer", "number", "boolean", "array"
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#enum: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<PropertySchema>>, // For array types
}

/// Tool with function declarations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "functionDeclarations")]
    pub function_declarations: Vec<FunctionDeclaration>,
}

/// Content in conversation (user/model/function)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub role: String, // "user", "model", "function"
    pub parts: Vec<Part>,
}

/// Part of a content (text, function call, or function response)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Part {
    Text {
        text: String,
    },
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: FunctionCall,
    },
    FunctionResponse {
        #[serde(rename = "functionResponse")]
        function_response: FunctionResponse,
    },
}

/// Function call from model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub args: serde_json::Value,
}

/// Function response to send back to model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionResponse {
    pub name: String,
    pub response: serde_json::Value,
}

// =========================================================================================================
// Chatbot Public API
// =========================================================================================================

/// Response from chatbot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatbotResponse {
    pub message: String,
    pub pending_confirmation: Option<PendingConfirmation>,
}

/// Pending action that requires user confirmation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingConfirmation {
    pub action_type: String, // "place_bet" or "sell_position"
    pub description: String,
    pub function_call: FunctionCall,
    pub selected: bool, // true = Yes, false = No (for UI toggle)
}

// =========================================================================================================
// AI Chatbot
// =========================================================================================================

pub struct AiChatbot {
    client: GeminiClient,
    personality: AiPersonality,
    db: DbPool,
    execution_engine: Arc<ExecutionEngine>,
    conversation_history: Vec<Content>,
    tools: Vec<Tool>,
    pending_confirmation: Option<PendingConfirmation>,
}

impl AiChatbot {
    pub fn new(
        client: GeminiClient,
        personality: AiPersonality,
        db: DbPool,
        execution_engine: Arc<ExecutionEngine>,
    ) -> Self {
        let tools = vec![Tool {
            function_declarations: vec![
                Self::web_search_declaration(),
                Self::search_markets_declaration(),
                Self::query_user_bets_declaration(),
                Self::place_bet_declaration(),
                Self::sell_position_declaration(),
                Self::analyze_market_declaration(),
            ],
        }];

        Self {
            client,
            personality,
            db,
            execution_engine,
            conversation_history: Vec::new(),
            tools,
            pending_confirmation: None,
        }
    }

    /// Send a message and get response (may require confirmation for sensitive actions)
    pub async fn send_message(&mut self, user_message: String) -> Result<ChatbotResponse> {
        // Add user message to history
        self.conversation_history.push(Content {
            role: "user".to_string(),
            parts: vec![Part::Text { text: user_message }],
        });

        // Keep only last 10 messages to avoid token limit
        if self.conversation_history.len() > 10 {
            self.conversation_history = self
                .conversation_history
                .iter()
                .skip(self.conversation_history.len() - 10)
                .cloned()
                .collect();
        }

        // Multi-turn loop: call model until we get a text response
        loop {
            // Serialize to JSON for API call
            let contents_json = serde_json::to_value(&self.conversation_history)?;
            let tools_json = serde_json::to_value(&self.tools)?;

            let response = self
                .client
                .generate_with_tools(
                    &contents_json,
                    &tools_json,
                    Some(self.personality.system_prompt()),
                )
                .await
                .context("Failed to call Gemini API")?;

            // Check if response has function call
            if let Some(function_call) = Self::extract_function_call(&response) {
                // Add model's function call to history
                self.conversation_history.push(Content {
                    role: "model".to_string(),
                    parts: vec![Part::FunctionCall {
                        function_call: function_call.clone(),
                    }],
                });

                // Check if this requires user confirmation
                if self.requires_confirmation(&function_call.name) {
                    self.pending_confirmation = Some(PendingConfirmation {
                        action_type: function_call.name.clone(),
                        description: self.format_confirmation_message(&function_call),
                        function_call,
                        selected: true, // Default to Yes
                    });

                    return Ok(ChatbotResponse {
                        message: "This action requires your confirmation.".to_string(),
                        pending_confirmation: self.pending_confirmation.clone(),
                    });
                }

                // Execute function immediately (non-sensitive)
                let result = self.execute_function(&function_call).await?;

                // Add function result to history
                self.conversation_history.push(Content {
                    role: "function".to_string(),
                    parts: vec![Part::FunctionResponse {
                        function_response: FunctionResponse {
                            name: function_call.name.clone(),
                            response: result,
                        },
                    }],
                });

                // Loop back - model will see the result and respond
                continue;
            }

            // No function call - extract text response
            if let Some(text) = Self::extract_text(&response) {
                self.conversation_history.push(Content {
                    role: "model".to_string(),
                    parts: vec![Part::Text { text: text.clone() }],
                });

                return Ok(ChatbotResponse {
                    message: text,
                    pending_confirmation: None,
                });
            }

            bail!("Unexpected response format from Gemini");
        }
    }

    /// Confirm and execute pending action
    pub async fn confirm_action(&mut self) -> Result<ChatbotResponse> {
        let confirmation = self
            .pending_confirmation
            .take()
            .context("No pending confirmation")?;

        // Execute the function
        let result = self.execute_function(&confirmation.function_call).await?;

        // Add function result to history
        self.conversation_history.push(Content {
            role: "function".to_string(),
            parts: vec![Part::FunctionResponse {
                function_response: FunctionResponse {
                    name: confirmation.function_call.name.clone(),
                    response: result,
                },
            }],
        });

        // Get final response from model
        let contents_json = serde_json::to_value(&self.conversation_history)?;
        let tools_json = serde_json::to_value(&self.tools)?;

        let response = self
            .client
            .generate_with_tools(
                &contents_json,
                &tools_json,
                Some(self.personality.system_prompt()),
            )
            .await?;

        if let Some(text) = Self::extract_text(&response) {
            self.conversation_history.push(Content {
                role: "model".to_string(),
                parts: vec![Part::Text { text: text.clone() }],
            });

            Ok(ChatbotResponse {
                message: text,
                pending_confirmation: None,
            })
        } else {
            bail!("Expected text response after function execution");
        }
    }

    /// Cancel pending action
    pub fn cancel_action(&mut self) {
        self.pending_confirmation = None;
    }

    /// Get conversation history (for display)
    pub fn get_history(&self) -> Vec<(String, String)> {
        // role, message
        self.conversation_history
            .iter()
            .filter_map(|content| {
                if let Some(Part::Text { text }) = content.parts.first() {
                    Some((content.role.clone(), text.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    fn extract_function_call(response: &serde_json::Value) -> Option<FunctionCall> {
        response
            .get("candidates")?
            .get(0)?
            .get("content")?
            .get("parts")?
            .get(0)?
            .get("functionCall")
            .and_then(|fc| serde_json::from_value(fc.clone()).ok())
    }

    fn extract_text(response: &serde_json::Value) -> Option<String> {
        response
            .get("candidates")?
            .get(0)?
            .get("content")?
            .get("parts")?
            .get(0)?
            .get("text")
            .and_then(|t| t.as_str())
            .map(String::from)
    }

    fn requires_confirmation(&self, function_name: &str) -> bool {
        matches!(function_name, "place_bet" | "sell_position")
    }

    fn format_confirmation_message(&self, function_call: &FunctionCall) -> String {
        match function_call.name.as_str() {
            "place_bet" => {
                let args = &function_call.args;
                format!(
                    "Place bet: {} on {} - ${} at {}%",
                    args.get("outcome").and_then(|v| v.as_str()).unwrap_or("?"),
                    args.get("market").and_then(|v| v.as_str()).unwrap_or("?"),
                    args.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    args.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0) * 100.0,
                )
            }
            "sell_position" => {
                let args = &function_call.args;
                format!(
                    "Sell position: {} on {}",
                    args.get("outcome").and_then(|v| v.as_str()).unwrap_or("?"),
                    args.get("market").and_then(|v| v.as_str()).unwrap_or("?"),
                )
            }
            _ => "Confirm action".to_string(),
        }
    }

    // ========================================================================
    // Function Execution (Placeholder implementations)
    // ========================================================================

    async fn execute_function(&self, function_call: &FunctionCall) -> Result<serde_json::Value> {
        match function_call.name.as_str() {
            "web_search" => self.web_search(&function_call.args).await,
            "search_markets" => self.search_markets(&function_call.args).await,
            "query_user_bets" => self.query_user_bets(&function_call.args).await,
            "place_bet" => self.place_bet(&function_call.args).await,
            "sell_position" => self.sell_position(&function_call.args).await,
            "analyze_market" => self.analyze_market(&function_call.args).await,
            _ => bail!("Unknown function: {}", function_call.name),
        }
    }

    /// Execute web search using Google Search (separate API call)
    async fn web_search(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        use serde_json::json;

        // Extract query from args
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .context("Missing query parameter")?;

        // Call Gemini with Google Search enabled (separate from function calling)
        let response = self
            .client
            .generate_with_google_search(query)
            .await
            .context("Failed to perform Google Search via Gemini")?;

        // Extract text from response
        if let Some(candidates) = response.get("candidates").and_then(|c| c.as_array()) {
            if let Some(first) = candidates.first() {
                if let Some(content) = first.get("content") {
                    if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                        if let Some(text_part) = parts.first() {
                            if let Some(text) = text_part.get("text").and_then(|t| t.as_str()) {
                                return Ok(json!({
                                    "success": true,
                                    "query": query,
                                    "results": text,
                                    "source": "Google Search via Gemini"
                                }));
                            }
                        }
                    }
                }
            }
        }

        // Fallback if parsing fails
        Ok(json!({
            "success": false,
            "query": query,
            "error": "Failed to parse Google Search results"
        }))
    }

    async fn search_markets(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        use serde_json::json;

        let keywords = args
            .get("keywords")
            .and_then(|v| v.as_str())
            .context("Missing keywords parameter")?;

        // Use MarketService to search Polymarket API
        let market_service = crate::trading::markets::MarketService::new();
        let markets = market_service
            .search_markets(keywords, 10)
            .await
            .context("Failed to search markets")?;

        // Format results for AI
        let markets_json: Vec<serde_json::Value> = markets
            .iter()
            .map(|m| {
                json!({
                    "id": m.id,
                    "question": m.question,
                    "volume": m.volume,
                    "outcomes": m.outcomes,
                    "prices": m.prices,
                })
            })
            .collect();

        Ok(json!({
            "success": true,
            "count": markets.len(),
            "markets": markets_json
        }))
    }

    async fn query_user_bets(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        use serde_json::json;

        let filter = args.get("filter").and_then(|v| v.as_str()).unwrap_or("all");

        // Query orders from database
        let query = if filter == "active" {
            "SELECT order_id, market_id, side, price, size, filled_size, status, created_at 
             FROM orders WHERE status = 'open' OR status = 'partial' ORDER BY created_at DESC LIMIT 50"
        } else if filter == "closed" {
            "SELECT order_id, market_id, side, price, size, filled_size, status, created_at 
             FROM orders WHERE status = 'filled' OR status = 'cancelled' ORDER BY created_at DESC LIMIT 50"
        } else {
            "SELECT order_id, market_id, side, price, size, filled_size, status, created_at 
             FROM orders ORDER BY created_at DESC LIMIT 50"
        };

        let rows = sqlx::query(query)
            .fetch_all(&self.db)
            .await
            .context("Failed to query orders")?;

        let mut bets = Vec::new();
        for row in rows {
            bets.push(json!({
                "order_id": row.get::<String, _>("order_id"),
                "market_id": row.get::<String, _>("market_id"),
                "side": row.get::<String, _>("side"),
                "price": row.get::<f64, _>("price"),
                "size": row.get::<f64, _>("size"),
                "filled_size": row.get::<f64, _>("filled_size"),
                "status": row.get::<String, _>("status"),
                "created_at": row.get::<i64, _>("created_at"),
            }));
        }

        // Get portfolio summary
        let portfolio_row = sqlx::query(
            "SELECT usdc_balance, total_value, realized_pnl, unrealized_pnl 
             FROM portfolio_snapshots ORDER BY timestamp DESC LIMIT 1",
        )
        .fetch_optional(&self.db)
        .await?;

        let portfolio = if let Some(row) = portfolio_row {
            json!({
                "usdc_balance": row.get::<f64, _>("usdc_balance"),
                "total_value": row.get::<f64, _>("total_value"),
                "realized_pnl": row.get::<f64, _>("realized_pnl"),
                "unrealized_pnl": row.get::<f64, _>("unrealized_pnl"),
                "total_pnl": row.get::<f64, _>("realized_pnl") + row.get::<f64, _>("unrealized_pnl"),
            })
        } else {
            json!({
                "usdc_balance": 0.0,
                "total_value": 0.0,
                "realized_pnl": 0.0,
                "unrealized_pnl": 0.0,
                "total_pnl": 0.0,
            })
        };

        Ok(json!({
            "success": true,
            "filter": filter,
            "count": bets.len(),
            "bets": bets,
            "portfolio": portfolio
        }))
    }

    async fn place_bet(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        use serde_json::json;

        let market = args
            .get("market")
            .and_then(|v| v.as_str())
            .context("Missing market parameter")?;
        let outcome = args
            .get("outcome")
            .and_then(|v| v.as_str())
            .context("Missing outcome parameter")?;
        let amount = args
            .get("amount")
            .and_then(|v| v.as_f64())
            .context("Missing amount parameter")?;
        let price = args
            .get("price")
            .and_then(|v| v.as_f64())
            .context("Missing price parameter")?;

        // Validate inputs
        if amount <= 0.0 {
            return Ok(json!({
                "success": false,
                "error": "Amount must be positive"
            }));
        }

        if !(0.0..=1.0).contains(&price) {
            return Ok(json!({
                "success": false,
                "error": "Price must be between 0.0 and 1.0"
            }));
        }

        // Resolve the outcome's CLOB token id and index so the order can be priced against
        // the real book. The AI passes the outcome by label; we map it to an index.
        let (outcome_index, token_id) = self.resolve_outcome(market, outcome).await;

        // Execute trade via ExecutionEngine (real path, with config-gated sim fallback).
        let request = crate::trading::TradeRequest {
            market_id: market.to_string(),
            token_id,
            outcome_index,
            outcome_label: outcome.to_string(),
            side: crate::data::OrderSide::Buy,
            size: amount,
            mode: crate::data::ExecutionMode::Real,
        };
        match self.execution_engine.place_trade(request).await {
            Ok(order) => Ok(json!({
                "success": true,
                "order_id": order.order_id,
                "market": market,
                "outcome": outcome,
                "amount": amount,
                "price": order.price,
                "mode": order.execution_mode.as_str(),
                "message": format!(
                    "Order placed ({}): {} @ ${:.2} on {}",
                    order.execution_mode.as_str(),
                    outcome,
                    order.price,
                    market
                )
            })),
            Err(e) => Ok(json!({
                "success": false,
                "error": format!("Failed to place bet: {}", e)
            })),
        }
    }

    /// Resolve an outcome label to its `(index, token_id)` for a market, fetching market
    /// metadata from Gamma. Falls back to `(0, "")` if the market or label can't be resolved
    /// — the execution engine then surfaces a clear "no token id" error.
    async fn resolve_outcome(&self, market_id: &str, outcome_label: &str) -> (usize, String) {
        let service = crate::trading::markets::MarketService::new();
        let Ok(Some(market)) = service.get_market(market_id).await else {
            return (0, String::new());
        };
        let index = market
            .outcomes
            .iter()
            .position(|o| o.eq_ignore_ascii_case(outcome_label))
            .unwrap_or(0);
        let token_id = market.token_ids.get(index).cloned().unwrap_or_default();
        (index, token_id)
    }

    async fn sell_position(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        use serde_json::json;

        let market = args
            .get("market")
            .and_then(|v| v.as_str())
            .context("Missing market parameter")?;
        let outcome = args
            .get("outcome")
            .and_then(|v| v.as_str())
            .context("Missing outcome parameter")?;
        // Amount of shares to sell; if omitted, the engine clamps to the held shares.
        let amount = args.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0);

        let (outcome_index, token_id) = self.resolve_outcome(market, outcome).await;

        match self
            .execution_engine
            .sell_position(
                market,
                &token_id,
                outcome_index,
                outcome,
                amount,
                crate::data::ExecutionMode::Real,
            )
            .await
        {
            Ok(order) => Ok(json!({
                "success": true,
                "order_id": order.order_id,
                "market": market,
                "outcome": outcome,
                "shares": order.size,
                "price": order.price,
                "mode": order.execution_mode.as_str(),
                "message": format!(
                    "Sell placed ({}): {:.4} shares of {} on {}",
                    order.execution_mode.as_str(),
                    order.size,
                    outcome,
                    market
                )
            })),
            Err(e) => Ok(json!({
                "success": false,
                "error": format!("Failed to sell position: {}", e)
            })),
        }
    }

    async fn analyze_market(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        use serde_json::json;

        let market_id = args
            .get("market_id")
            .and_then(|v| v.as_str())
            .context("Missing market_id parameter")?;

        // Get market info from database or Polymarket API
        let market_service = crate::trading::markets::MarketService::new();
        let market_opt = market_service
            .get_market(market_id)
            .await
            .context("Failed to fetch market")?;

        let market = match market_opt {
            Some(m) => m,
            None => {
                return Ok(json!({
                    "success": false,
                    "error": format!("Market {} not found", market_id)
                }));
            }
        };

        // Get spike detection data
        let spike_data = sqlx::query(
            "SELECT velocity, volume_delta, time_delta, timestamp 
             FROM volume_velocity_events 
             WHERE market_id = ? 
             ORDER BY timestamp DESC LIMIT 5",
        )
        .bind(market_id)
        .fetch_all(&self.db)
        .await
        .unwrap_or_default();

        let spikes: Vec<serde_json::Value> = spike_data
            .iter()
            .map(|row| {
                json!({
                    "velocity": row.get::<f64, _>("velocity"),
                    "volume_delta": row.get::<f64, _>("volume_delta"),
                    "time_delta": row.get::<f64, _>("time_delta"),
                    "timestamp": row.get::<i64, _>("timestamp"),
                })
            })
            .collect();

        // Get order book imbalance
        let obi_row = sqlx::query(
            "SELECT bids_volume, asks_volume, best_bid, best_ask, timestamp 
             FROM orderbook_snapshots 
             WHERE market_id = ? 
             ORDER BY timestamp DESC LIMIT 1",
        )
        .bind(market_id)
        .fetch_optional(&self.db)
        .await?;

        let order_book = if let Some(row) = obi_row {
            let bids_vol = row.get::<f64, _>("bids_volume");
            let asks_vol = row.get::<f64, _>("asks_volume");
            let obi = if (bids_vol + asks_vol) > 0.0 {
                (bids_vol - asks_vol) / (bids_vol + asks_vol)
            } else {
                0.0
            };

            json!({
                "bids_volume": bids_vol,
                "asks_volume": asks_vol,
                "obi": obi,
                "best_bid": row.get::<Option<f64>, _>("best_bid"),
                "best_ask": row.get::<Option<f64>, _>("best_ask"),
            })
        } else {
            json!({
                "bids_volume": 0.0,
                "asks_volume": 0.0,
                "obi": 0.0,
                "best_bid": null,
                "best_ask": null,
            })
        };

        // Get AI recommendation if available
        let ai_rec = sqlx::query(
            "SELECT action, confidence, reasoning, analysis 
             FROM ai_recommendations 
             WHERE market_id = ? 
             ORDER BY timestamp DESC LIMIT 1",
        )
        .bind(market_id)
        .fetch_optional(&self.db)
        .await?;

        let ai_recommendation = if let Some(row) = ai_rec {
            json!({
                "action": row.get::<String, _>("action"),
                "confidence": row.get::<f64, _>("confidence"),
                "reasoning": row.get::<String, _>("reasoning"),
            })
        } else {
            json!(null)
        };

        Ok(json!({
            "success": true,
            "market_id": market.id,
            "question": market.question,
            "volume": market.volume,
            "outcomes": market.outcomes,
            "prices": market.prices,
            "spikes": spikes,
            "spike_count": spikes.len(),
            "order_book": order_book,
            "ai_recommendation": ai_recommendation,
        }))
    }

    // ========================================================================
    // Function Declarations (Tools)
    // ========================================================================

    fn web_search_declaration() -> FunctionDeclaration {
        FunctionDeclaration {
            name: "web_search".to_string(),
            description: "Search the web for news, articles, and information".to_string(),
            parameters: JsonSchema {
                schema_type: "object".to_string(),
                properties: HashMap::from([(
                    "query".to_string(),
                    PropertySchema {
                        property_type: "string".to_string(),
                        description: "Search query string".to_string(),
                        r#enum: None,
                        items: None,
                    },
                )]),
                required: vec!["query".to_string()],
            },
        }
    }

    fn search_markets_declaration() -> FunctionDeclaration {
        FunctionDeclaration {
            name: "search_markets".to_string(),
            description: "Search for prediction markets on Polymarket".to_string(),
            parameters: JsonSchema {
                schema_type: "object".to_string(),
                properties: HashMap::from([(
                    "keywords".to_string(),
                    PropertySchema {
                        property_type: "string".to_string(),
                        description: "Keywords to search for markets".to_string(),
                        r#enum: None,
                        items: None,
                    },
                )]),
                required: vec!["keywords".to_string()],
            },
        }
    }

    fn query_user_bets_declaration() -> FunctionDeclaration {
        FunctionDeclaration {
            name: "query_user_bets".to_string(),
            description: "Query the user's current betting positions and history".to_string(),
            parameters: JsonSchema {
                schema_type: "object".to_string(),
                properties: HashMap::from([(
                    "filter".to_string(),
                    PropertySchema {
                        property_type: "string".to_string(),
                        description: "Optional filter (e.g., 'active', 'closed', keyword)"
                            .to_string(),
                        r#enum: None,
                        items: None,
                    },
                )]),
                required: vec![],
            },
        }
    }

    fn place_bet_declaration() -> FunctionDeclaration {
        FunctionDeclaration {
            name: "place_bet".to_string(),
            description: "Place a bet on a prediction market (REQUIRES USER CONFIRMATION)"
                .to_string(),
            parameters: JsonSchema {
                schema_type: "object".to_string(),
                properties: HashMap::from([
                    (
                        "market".to_string(),
                        PropertySchema {
                            property_type: "string".to_string(),
                            description: "Market ID or question".to_string(),
                            r#enum: None,
                            items: None,
                        },
                    ),
                    (
                        "outcome".to_string(),
                        PropertySchema {
                            property_type: "string".to_string(),
                            description: "Outcome to bet on (Yes/No)".to_string(),
                            r#enum: Some(vec!["Yes".to_string(), "No".to_string()]),
                            items: None,
                        },
                    ),
                    (
                        "amount".to_string(),
                        PropertySchema {
                            property_type: "number".to_string(),
                            description: "Amount in USDC to bet".to_string(),
                            r#enum: None,
                            items: None,
                        },
                    ),
                    (
                        "price".to_string(),
                        PropertySchema {
                            property_type: "number".to_string(),
                            description: "Price to bet at (0.0-1.0)".to_string(),
                            r#enum: None,
                            items: None,
                        },
                    ),
                ]),
                required: vec![
                    "market".to_string(),
                    "outcome".to_string(),
                    "amount".to_string(),
                    "price".to_string(),
                ],
            },
        }
    }

    fn sell_position_declaration() -> FunctionDeclaration {
        FunctionDeclaration {
            name: "sell_position".to_string(),
            description: "Sell an existing betting position (REQUIRES USER CONFIRMATION)"
                .to_string(),
            parameters: JsonSchema {
                schema_type: "object".to_string(),
                properties: HashMap::from([
                    (
                        "market".to_string(),
                        PropertySchema {
                            property_type: "string".to_string(),
                            description: "Market ID or question".to_string(),
                            r#enum: None,
                            items: None,
                        },
                    ),
                    (
                        "outcome".to_string(),
                        PropertySchema {
                            property_type: "string".to_string(),
                            description: "Outcome position to sell".to_string(),
                            r#enum: None,
                            items: None,
                        },
                    ),
                ]),
                required: vec!["market".to_string(), "outcome".to_string()],
            },
        }
    }

    fn analyze_market_declaration() -> FunctionDeclaration {
        FunctionDeclaration {
            name: "analyze_market".to_string(),
            description:
                "Perform detailed analysis on a specific market using spike detection and OBI"
                    .to_string(),
            parameters: JsonSchema {
                schema_type: "object".to_string(),
                properties: HashMap::from([(
                    "market_id".to_string(),
                    PropertySchema {
                        property_type: "string".to_string(),
                        description: "Market ID to analyze".to_string(),
                        r#enum: None,
                        items: None,
                    },
                )]),
                required: vec!["market_id".to_string()],
            },
        }
    }
}
