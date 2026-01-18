// exchange/utils.rs
// Utility functions for Hyperliquid exchange

use super::ExchangeError;
use rust_decimal::Decimal;
use std::str::FromStr;

/// Validate symbol format for Hyperliquid
pub fn validate_hyperliquid_symbol(symbol: &str) -> Result<String, ExchangeError> {
    if symbol.is_empty() {
        return Err(ExchangeError::InvalidSymbol(
            "Symbol cannot be empty".to_string(),
        ));
    }

    let symbol = symbol.to_uppercase();

    // Basic validation: should be alphanumeric and reasonable length
    if !symbol.chars().all(char::is_alphanumeric) {
        return Err(ExchangeError::InvalidSymbol(format!(
            "Symbol '{}' contains invalid characters",
            symbol
        )));
    }

    if symbol.len() < 2 || symbol.len() > 10 {
        return Err(ExchangeError::InvalidSymbol(format!(
            "Symbol '{}' has invalid length",
            symbol
        )));
    }

    Ok(symbol)
}

/// Parse price string to Decimal
pub fn parse_price(price_str: &str) -> Result<Decimal, ExchangeError> {
    Decimal::from_str(price_str)
        .map_err(|e| ExchangeError::ParseError(format!("Invalid price '{}': {}", price_str, e)))
}

/// Parse size/quantity string to Decimal
pub fn parse_size(size_str: &str) -> Result<Decimal, ExchangeError> {
    Decimal::from_str(size_str)
        .map_err(|e| ExchangeError::ParseError(format!("Invalid size '{}': {}", size_str, e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_validation() {
        assert!(validate_hyperliquid_symbol("BTC").is_ok());
        assert!(validate_hyperliquid_symbol("ETH").is_ok());
        assert!(validate_hyperliquid_symbol("").is_err());
        assert!(validate_hyperliquid_symbol("BTC-USD").is_err());
    }
}
