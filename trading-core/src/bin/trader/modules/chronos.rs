// E:\MBCT\trading-core\src\bin\trader\modules\chronos.rs
// THE ALLIANCE - MBCT Chronos v2.0 (Trader Integration)
// Fokus: Fibonacci Time-Horizons & Peak Detection

use super::physicist::PhysicsState;
use super::regime::RegimeState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MBCTFullRecord {
    pub timestamp: u128,
    pub symbol: String,
    pub physics: PhysicsState,
    pub regime: RegimeState,
    pub ret_3s: Option<f64>,
    pub ret_5s: Option<f64>,
    pub ret_8s: Option<f64>,
    pub ret_13s: Option<f64>,
    pub ret_21s: Option<f64>,
    pub ret_34s: Option<f64>,
    pub ret_55s: Option<f64>,
    pub ret_89s: Option<f64>,
    pub ret_144s: Option<f64>,
    pub ret_233s: Option<f64>,
    pub ret_377s: Option<f64>,
    pub z_entropy_21s: f64,
    pub z_pressure_21s: f64,
    pub z_nrg_21s: f64,
    pub z_entropy_34s: f64,
    pub z_pressure_34s: f64,
    pub z_nrg_34s: f64,
    pub is_complete: bool,
    #[serde(skip, default = "Instant::now")]
    #[allow(dead_code)]
    pub created_at: Instant,
}

struct PeakCandidate {
    physics: PhysicsState,
    regime: RegimeState,
    last_update: Instant,
}

pub struct Chronos {
    pending_records: HashMap<String, Vec<MBCTFullRecord>>,
    active_peaks: HashMap<String, PeakCandidate>,
}

impl Chronos {
    pub fn new() -> Self {
        Self {
            pending_records: HashMap::new(),
            active_peaks: HashMap::new(),
        }
    }

    /// Überwacht Symmetrie-Extreme (Erdbeben vs Rippel)
    pub fn observe_potential_hit(
        &mut self,
        symbol: &str,
        physics: &PhysicsState,
        regime: &RegimeState,
        l_floor: f64,
        s_ceiling: f64,
    ) -> bool {
        let current_sym_score = regime.symmetry_score;
        if current_sym_score < 0.001 {
            return false;
        }

        let is_triggering = current_sym_score < l_floor || current_sym_score > s_ceiling;

        if is_triggering {
            if let Some(peak) = self.active_peaks.get_mut(symbol) {
                let is_more_extreme = if current_sym_score < l_floor {
                    current_sym_score < peak.regime.symmetry_score
                } else {
                    current_sym_score > peak.regime.symmetry_score
                };

                if is_more_extreme {
                    peak.physics = physics.clone();
                    peak.regime = regime.clone();
                }
                peak.last_update = Instant::now();
            } else {
                self.active_peaks.insert(
                    symbol.to_string(),
                    PeakCandidate {
                        physics: physics.clone(),
                        regime: regime.clone(),
                        last_update: Instant::now(),
                    },
                );
            }
        } else {
            if let Some(peak) = self.active_peaks.remove(symbol) {
                self.finalize_peak(symbol, peak);
                return true;
            }
        }

        let mut force_finalize = false;
        if let Some(peak) = self.active_peaks.get(symbol) {
            if peak.last_update.elapsed().as_secs() > 10 {
                force_finalize = true;
            }
        }

        if force_finalize {
            if let Some(peak) = self.active_peaks.remove(symbol) {
                self.finalize_peak(symbol, peak);
                return true;
            }
        }

        false
    }

    fn finalize_peak(&mut self, symbol: &str, peak: PeakCandidate) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let record = MBCTFullRecord {
            timestamp: now,
            symbol: symbol.to_string(),
            physics: peak.physics,
            regime: peak.regime,
            ret_3s: None,
            ret_5s: None,
            ret_8s: None,
            ret_13s: None,
            ret_21s: None,
            ret_34s: None,
            ret_55s: None,
            ret_89s: None,
            ret_144s: None,
            ret_233s: None,
            ret_377s: None,
            z_entropy_21s: 0.0,
            z_pressure_21s: 0.0,
            z_nrg_21s: 0.0,
            z_entropy_34s: 0.0,
            z_pressure_34s: 0.0,
            z_nrg_34s: 0.0,
            is_complete: false,
            created_at: Instant::now(),
        };

        self.pending_records
            .entry(symbol.to_string())
            .or_insert_with(Vec::new)
            .push(record);
    }

    #[allow(dead_code)]
    pub fn update_and_flush(
        &mut self,
        symbol: &str,
        current_price: f64,
        z_21: (f64, f64, f64),
        z_34: (f64, f64, f64),
    ) -> Vec<MBCTFullRecord> {
        let mut completed = Vec::new();
        if let Some(records) = self.pending_records.get_mut(symbol) {
            let now = Instant::now();
            for r in records.iter_mut() {
                if r.is_complete {
                    continue;
                }
                let elapsed = now.duration_since(r.created_at).as_secs();
                let p0 = r.physics.price;
                let calc_ret = |p_s: f64, p_n: f64| {
                    if p_s <= 0.0 {
                        0.0
                    } else {
                        ((p_n - p_s) / p_s) * 100.0
                    }
                };

                if r.ret_3s.is_none() && elapsed >= 3 {
                    r.ret_3s = Some(calc_ret(p0, current_price));
                }
                if r.ret_5s.is_none() && elapsed >= 5 {
                    r.ret_5s = Some(calc_ret(p0, current_price));
                }
                if r.ret_8s.is_none() && elapsed >= 8 {
                    r.ret_8s = Some(calc_ret(p0, current_price));
                }
                if r.ret_13s.is_none() && elapsed >= 13 {
                    r.ret_13s = Some(calc_ret(p0, current_price));
                }
                if r.ret_21s.is_none() && elapsed >= 21 {
                    r.ret_21s = Some(calc_ret(p0, current_price));
                    r.z_entropy_21s = z_21.0;
                    r.z_pressure_21s = z_21.1;
                    r.z_nrg_21s = z_21.2;
                }
                if r.ret_34s.is_none() && elapsed >= 34 {
                    r.ret_34s = Some(calc_ret(p0, current_price));
                    r.z_entropy_34s = z_34.0;
                    r.z_pressure_34s = z_34.1;
                    r.z_nrg_34s = z_34.2;
                }
                if r.ret_55s.is_none() && elapsed >= 55 {
                    r.ret_55s = Some(calc_ret(p0, current_price));
                }
                if r.ret_89s.is_none() && elapsed >= 89 {
                    r.ret_89s = Some(calc_ret(p0, current_price));
                }
                if r.ret_144s.is_none() && elapsed >= 144 {
                    r.ret_144s = Some(calc_ret(p0, current_price));
                }
                if r.ret_233s.is_none() && elapsed >= 233 {
                    r.ret_233s = Some(calc_ret(p0, current_price));
                }
                if r.ret_377s.is_none() && elapsed >= 377 {
                    r.ret_377s = Some(calc_ret(p0, current_price));
                    r.is_complete = true;
                }
            }
            records.retain(|r| {
                if r.is_complete {
                    completed.push(r.clone());
                    false
                } else {
                    true
                }
            });
        }
        completed
    }

    #[allow(dead_code)]
    pub fn get_pending_count(&self) -> usize {
        self.pending_records
            .values()
            .map(|v| v.len())
            .sum::<usize>()
            + self.active_peaks.len()
    }
}
