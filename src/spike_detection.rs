use crate::database::DbPool;
use crate::types::VolumeVelocityEvent;
use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;

/// Spike detection algorithms for identifying trading opportunities
pub struct SpikeDetector {
    db: DbPool,
    // Track previous volume states per market
    volume_history: HashMap<String, VolumeHistory>,
    // Configuration thresholds
    volume_velocity_threshold: f64,
    obi_threshold: f64,
}

#[derive(Debug, Clone)]
struct VolumeHistory {
    last_volume: f64,
    last_timestamp: i64,
}

impl SpikeDetector {
    pub fn new(db: DbPool, volume_velocity_threshold: f64, obi_threshold: f64) -> Self {
        Self {
            db,
            volume_history: HashMap::new(),
            volume_velocity_threshold,
            obi_threshold,
        }
    }
    
    /// Calculate volume velocity: V_v = Delta_Volume / Delta_t
    /// Returns true if velocity exceeds threshold
    pub async fn check_volume_velocity(
        &mut self,
        market_id: &str,
        current_volume: f64,
    ) -> Result<Option<VolumeVelocityEvent>> {
        let now = Utc::now().timestamp();
        
        // Get previous state for this market
        let event = if let Some(history) = self.volume_history.get(market_id) {
            let volume_delta = current_volume - history.last_volume;
            let time_delta = (now - history.last_timestamp) as f64;
            
            if time_delta > 0.0 {
                let velocity = volume_delta / time_delta;
                
                // Check if velocity exceeds threshold
                if velocity.abs() > self.volume_velocity_threshold {
                    Some(VolumeVelocityEvent {
                        market_id: market_id.to_string(),
                        velocity,
                        volume_delta,
                        time_delta,
                        timestamp: now,
                    })
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        
        // Update history
        self.volume_history.insert(
            market_id.to_string(),
            VolumeHistory {
                last_volume: current_volume,
                last_timestamp: now,
            },
        );
        
        // If we detected a spike, save it to database
        if let Some(ref evt) = event {
            self.save_velocity_event(evt).await?;
        }
        
        Ok(event)
    }
    
    /// Calculate order book imbalance: OBI = (V_bids - V_asks) / (V_bids + V_asks)
    /// Returns OBI value between -1 and 1
    pub fn calculate_order_book_imbalance(
        &self,
        bids_volume: f64,
        asks_volume: f64,
    ) -> f64 {
        let total_volume = bids_volume + asks_volume;
        if total_volume == 0.0 {
            return 0.0;
        }
        (bids_volume - asks_volume) / total_volume
    }
    
    /// Check if OBI indicates a significant imbalance
    pub fn is_significant_imbalance(&self, obi: f64) -> bool {
        obi.abs() > self.obi_threshold
    }
    
    async fn save_velocity_event(&self, event: &VolumeVelocityEvent) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO volume_velocity_events 
            (market_id, velocity, volume_delta, time_delta, timestamp)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&event.market_id)
        .bind(event.velocity)
        .bind(event.volume_delta)
        .bind(event.time_delta)
        .bind(event.timestamp)
        .execute(&self.db)
        .await?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_obi_calculation() {
        let detector = SpikeDetector {
            db: unimplemented!(),
            volume_history: HashMap::new(),
            volume_velocity_threshold: 1000.0,
            obi_threshold: 0.3,
        };
        
        // Equal volumes = 0 imbalance
        assert_eq!(detector.calculate_order_book_imbalance(100.0, 100.0), 0.0);
        
        // All bids = 1.0
        assert_eq!(detector.calculate_order_book_imbalance(100.0, 0.0), 1.0);
        
        // All asks = -1.0
        assert_eq!(detector.calculate_order_book_imbalance(0.0, 100.0), -1.0);
        
        // 60/40 split
        let obi = detector.calculate_order_book_imbalance(60.0, 40.0);
        assert!((obi - 0.2).abs() < 0.01);
    }
}
