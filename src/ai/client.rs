//! Google Gemini API client for AI-powered market analysis.

// =========================================================================================================
// Imports
// =========================================================================================================

use std::num::NonZeroU32;
use std::sync::Arc;

use anyhow::{Context, Result};
use governor::{Quota, RateLimiter};
use serde::{Deserialize, Serialize};

// =========================================================================================================
// Constants
// =========================================================================================================

const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";
const GEMINI_MODEL: &str = "gemini-2.0-flash";

// =========================================================================================================
// Types
// =========================================================================================================

/// Response from Gemini API.
#[derive(Debug, Clone)]
pub struct AiResponse {
    pub text: String,
    pub model: String,
}

/// Gemini API request structures.
#[derive(Debug, Serialize)]
struct GenerateContentRequest {
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<SystemInstruction>,
}

#[derive(Debug, Serialize)]
struct SystemInstruction {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct Part {
    text: String,
}

/// Gemini API response structures
#[derive(Debug, Deserialize)]
struct GenerateContentResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: ContentResponse,
}

#[derive(Debug, Deserialize)]
struct ContentResponse {
    parts: Vec<PartResponse>,
}

#[derive(Debug, Deserialize)]
struct PartResponse {
    text: String,
}

/// Client for Google Gemini API.
pub struct GeminiClient {
    api_key: String,
    client: reqwest::Client,
    rate_limiter: Arc<
        RateLimiter<
            governor::state::NotKeyed,
            governor::state::InMemoryState,
            governor::clock::DefaultClock,
        >,
    >,
}

// =========================================================================================================
// Implementation
// =========================================================================================================

impl GeminiClient {
    /// Create a new Gemini client with API key
    pub fn new(api_key: String) -> Self {
        // Free tier: 60 requests per minute
        let quota = Quota::per_minute(NonZeroU32::new(60).unwrap());
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        Self {
            api_key,
            client: reqwest::Client::new(),
            rate_limiter,
        }
    }

    /// Generate content from a prompt with optional system instruction
    pub async fn generate(
        &self,
        prompt: &str,
        system_instruction: Option<&str>,
    ) -> Result<AiResponse> {
        // Rate limit check
        self.rate_limiter.until_ready().await;

        let url = format!(
            "{}/{}:generateContent?key={}",
            GEMINI_API_BASE, GEMINI_MODEL, self.api_key
        );

        let mut request = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: prompt.to_string(),
                }],
            }],
            system_instruction: None,
        };

        if let Some(instruction) = system_instruction {
            request.system_instruction = Some(SystemInstruction {
                parts: vec![Part {
                    text: instruction.to_string(),
                }],
            });
        }

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Gemini API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Gemini API request failed with status {}: {}",
                status,
                error_text
            );
        }

        let api_response: GenerateContentResponse = response
            .json()
            .await
            .context("Failed to parse Gemini API response")?;

        let text = api_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .unwrap_or_default();

        Ok(AiResponse {
            text,
            model: GEMINI_MODEL.to_string(),
        })
    }

    /// Generate content with retry logic for transient failures
    pub async fn generate_with_retry(
        &self,
        prompt: &str,
        system_instruction: Option<&str>,
        max_retries: u32,
    ) -> Result<AiResponse> {
        let mut last_error = None;

        for attempt in 0..=max_retries {
            match self.generate(prompt, system_instruction).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries {
                        // Exponential backoff
                        let delay = std::time::Duration::from_secs(2u64.pow(attempt));
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }

    /// Generate content with tools (function calling)
    pub async fn generate_with_tools(
        &self,
        contents: &serde_json::Value,
        tools: &serde_json::Value,
        system_instruction: Option<&str>,
    ) -> Result<serde_json::Value> {
        // Rate limit check
        self.rate_limiter.until_ready().await;

        let url = format!(
            "{}/{}:generateContent?key={}",
            GEMINI_API_BASE, GEMINI_MODEL, self.api_key
        );

        let mut request_body = serde_json::json!({
            "contents": contents,
            "tools": tools,
        });

        if let Some(instruction) = system_instruction {
            request_body["systemInstruction"] = serde_json::json!({
                "parts": [{"text": instruction}]
            });
        }

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to Gemini API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Gemini API request failed with status {}: {}",
                status,
                error_text
            );
        }

        let json_response: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Gemini API response")?;

        Ok(json_response)
    }

    /// Generate content with Google Search grounding
    /// Note: Cannot be used simultaneously with function calling
    pub async fn generate_with_google_search(&self, query: &str) -> Result<serde_json::Value> {
        // Rate limit check
        self.rate_limiter.until_ready().await;

        let url = format!(
            "{}/{}:generateContent?key={}",
            GEMINI_API_BASE, GEMINI_MODEL, self.api_key
        );

        let request_body = serde_json::json!({
            "contents": [{
                "role": "user",
                "parts": [{"text": query}]
            }],
            "tools": [{
                "googleSearch": {}
            }]
        });

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to send Google Search request to Gemini API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Gemini API Google Search request failed with status {}: {}",
                status,
                error_text
            );
        }

        let json_response: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Gemini Google Search response")?;

        Ok(json_response)
    }
}

// =========================================================================================================
// Tests
// =========================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires real API key
    async fn test_gemini_api_integration() {
        let api_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set");
        let client = GeminiClient::new(api_key);

        let response = client
            .generate("Say 'Hello, World!' in exactly 2 words.", None)
            .await
            .unwrap();

        assert!(!response.text.is_empty());
        assert_eq!(response.model, GEMINI_MODEL);
    }

    #[test]
    fn test_client_creation() {
        let client = GeminiClient::new("test_key".to_string());
        assert_eq!(client.api_key, "test_key");
    }
}
