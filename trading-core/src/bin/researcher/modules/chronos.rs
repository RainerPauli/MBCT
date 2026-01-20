// E:\MBCT\trading-core\src\bin\researcher\modules\chronos.rs
// THE ALLIANCE - MBCT Chronos Modul
// Fokus: Fibonacci-Zeitfenster & Future-Return-Validierung

use crate::modules::physicist::PhysicsState;
use crate::modules::regime::RegimeState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MBCTFullRecord {
    pub timestamp: u128,
    pub symbol: String,
    pub physics: PhysicsState,
    pub regime: RegimeState,
    pub ret_3s: Option<f64>,
    pub ret_8s: Option<f64>,
    pub ret_21s: Option<f64>,
    pub ret_55s: Option<f64>,
    pub ret_89s: Option<f64>,
    pub is_complete: bool,
    #[serde(skip, default = "Instant::now")]
    pub created_at: Instant,
}

pub struct Chronos {
    pending_records: HashMap<String, Vec<MBCTFullRecord>>,
    fibonacci_windows: Vec<u64>, 
}

impl Chronos {
    pub fn new() -> Self {
        Self {
            pending_records: HashMap::new(),
            fibonacci_windows: vec![3, 8, 21, 55, 89],
        }
    }

    pub fn register_observation(&mut self, symbol: &str, physics: PhysicsState, regime: RegimeState) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let record = MBCTFullRecord {
            timestamp: now,
            symbol: symbol.to_string(),
            physics,
            regime,
            ret_3s: None,
            ret_8s: None,
            ret_21s: None,
            ret_55s: None,
            ret_89s: None,
            is_complete: false,
            created_at: Instant::now(),
        };

        self.pending_records.entry(symbol.to_string()).or_default().push(record);
    }

    pub fn update_and_flush(&mut self, symbol: &str, current_price: f64) -> Vec<MBCTFullRecord> {
        let mut completed = Vec::new();
        let windows = &self.fibonacci_windows;
        
        if let Some(records) = self.pending_records.get_mut(symbol) {
            let now = Instant::now();
            for record in records.iter_mut() {
                let elapsed = now.duration_since(record.created_at).as_secs();
                let entry_p = record.physics.price;

                if record.ret_3s.is_none() && elapsed >= windows[0] {
                    record.ret_3s = Some(Self::calculate_return(entry_p, current_price));
                }
                if record.ret_8s.is_none() && elapsed >= windows[1] {
                    record.ret_8s = Some(Self::calculate_return(entry_p, current_price));
                }
                if record.ret_21s.is_none() && elapsed >= windows[2] {
                    record.ret_21s = Some(Self::calculate_return(entry_p, current_price));
                }
                if record.ret_55s.is_none() && elapsed >= windows[3] {
                    record.ret_55s = Some(Self::calculate_return(entry_p, current_price));
                }
                if record.ret_89s.is_none() && elapsed >= windows[4] {
                    record.ret_89s = Some(Self::calculate_return(entry_p, current_price));
                    record.is_complete = true;
                }
            }
            let mut i = 0;
            while i < records.len() {
                if records[i].is_complete {
                    completed.push(records.remove(i));
                } else {
                    i += 1;
                }
            }
        }
        completed
    }

    fn calculate_return(entry_price: f64, current_price: f64) -> f64 {
        if entry_price <= 0.0 { return 0.0; }
        ((current_price - entry_price) / entry_price) * 100.0
    }
}