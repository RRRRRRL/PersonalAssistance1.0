//! Market Data Engine — inspired by NautilusTrader's DataEngine + adapter pattern.
//!
//! Provides a unified interface for fetching market data from multiple venues:
//! - **Yahoo Finance**: Equities, ETFs, indices
//! - **CoinGecko**: Cryptocurrencies
//!
//! The cache layer stores the latest quote per symbol in SQLite (`price_cache`).
//! A TTL mechanism avoids hammering upstream APIs.

use serde::Deserialize;

/// Unified market quote produced by any adapter.
#[derive(Debug, Clone)]
pub struct MarketQuote {
    pub symbol: String,
    pub price: f64,
    pub change_percent: Option<f64>,
    pub volume: Option<f64>,
    pub market_cap: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
}


// ─── Yahoo Finance adapter ───────────────────────────────────────────────────

/// Minimal Yahoo Finance quote response (v8 chart API).
#[derive(Debug, Deserialize)]
struct YahooChartResponse {
    chart: Option<YahooChart>,
}

#[derive(Debug, Deserialize)]
struct YahooChart {
    result: Option<Vec<YahooResult>>,
}

#[derive(Debug, Deserialize)]
struct YahooResult {
    meta: YahooMeta,
}

#[derive(Debug, Deserialize)]
struct YahooMeta {
    #[serde(rename = "regularMarketPrice")]
    regular_market_price: Option<f64>,
    #[serde(rename = "previousClose")]
    previous_close: Option<f64>,
    #[serde(rename = "regularMarketDayHigh")]
    regular_market_day_high: Option<f64>,
    #[serde(rename = "regularMarketDayLow")]
    regular_market_day_low: Option<f64>,
    symbol: Option<String>,
}

/// Fetch quotes for equity symbols from Yahoo Finance.
pub async fn fetch_yahoo_quotes(symbols: &[String]) -> Vec<MarketQuote> {
    if symbols.is_empty() {
        return vec![];
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    let mut quotes = Vec::new();

    // Fetch each symbol individually for reliability
    for symbol in symbols {
        let url = format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{}?range=1d&interval=1d",
            symbol
        );

        match client.get(&url).header("User-Agent", "Mozilla/5.0").send().await {
            Ok(resp) => {
                if let Ok(data) = resp.json::<YahooChartResponse>().await {
                    if let Some(chart) = data.chart {
                        if let Some(results) = chart.result {
                            if let Some(result) = results.first() {
                                let meta = &result.meta;
                                let price = meta.regular_market_price.unwrap_or(0.0);
                                let prev = meta.previous_close.unwrap_or(price);
                                let change = if prev > 0.0 {
                                    Some(((price - prev) / prev) * 100.0)
                                } else {
                                    None
                                };

                                quotes.push(MarketQuote {
                                    symbol: symbol.clone(),
                                    price,
                                    change_percent: change,
                                    volume: None,
                                    market_cap: None,
                                    high: meta.regular_market_day_high,
                                    low: meta.regular_market_day_low,
                                });
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("[market_data] Yahoo fetch failed for {}: {}", symbol, e);
            }
        }
    }

    quotes
}

// ─── CoinGecko adapter ──────────────────────────────────────────────────────

/// CoinGecko simple/price response.
#[derive(Debug, Deserialize)]
struct CoinGeckoPrice {
    usd: Option<f64>,
    usd_24h_change: Option<f64>,
    usd_market_cap: Option<f64>,
    usd_24h_vol: Option<f64>,
}

/// Fetch quotes for crypto symbols from CoinGecko (free API, no key required).
/// Symbols should be CoinGecko IDs (e.g., "bitcoin", "ethereum").
pub async fn fetch_coingecko_quotes(symbols: &[String]) -> Vec<MarketQuote> {
    if symbols.is_empty() {
        return vec![];
    }

    let ids = symbols.join(",");
    let url = format!(
        "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd&include_24hr_change=true&include_market_cap=true&include_24hr_vol=true",
        ids
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    let mut quotes = Vec::new();

    match client.get(&url).header("Accept", "application/json").send().await {
        Ok(resp) => {
            if let Ok(data) = resp.json::<std::collections::HashMap<String, CoinGeckoPrice>>().await {
                for symbol in symbols {
                    if let Some(price_data) = data.get(symbol.as_str()) {
                        quotes.push(MarketQuote {
                            symbol: symbol.clone(),
                            price: price_data.usd.unwrap_or(0.0),
                            change_percent: price_data.usd_24h_change,
                            volume: price_data.usd_24h_vol,
                            market_cap: price_data.usd_market_cap,
                            high: None,
                            low: None,
                        });
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("[market_data] CoinGecko fetch failed: {}", e);
        }
    }

    quotes
}

