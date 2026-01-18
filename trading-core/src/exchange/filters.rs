// File: src/exchange/filters.rs
// Placeholder for exchange-specific filters (e.g., price filters, lot size filters)
// To be implemented when order execution is needed

use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct ExchangeFilters {
    pub min_price: Option<Decimal>,
    pub max_price: Option<Decimal>,
    pub tick_size: Option<Decimal>,
    pub min_qty: Option<Decimal>,
    pub max_qty: Option<Decimal>,
    pub step_size: Option<Decimal>,
}

impl Default for ExchangeFilters {
    fn default() -> Self {
        Self {
            min_price: None,
            max_price: None,
            tick_size: None,
            min_qty: None,
            max_qty: None,
            step_size: None,
        }
    }
}
