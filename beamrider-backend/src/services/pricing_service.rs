#[derive(Debug, Clone, Default)]
pub struct PricingService;

impl PricingService {
    pub const fn normalize_quote_symbol(symbol: &str) -> &str {
        match symbol.as_bytes() {
            b"USD" => "USD",
            b"USDC" => "USD",
            b"CUSD" => "USD",
            _ => symbol,
        }
    }
}
