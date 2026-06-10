use serde::Deserialize;

// Spot candlestick: array of arrays
// ["timestamp_str","quote_volume","close","high","low","open","volume","is_window_closed"]
pub type GateSpotCandleRaw = Vec<String>;

// Futures candlestick
#[derive(Debug, Deserialize)]
pub struct GateFuturesCandleRaw {
    pub t: i64,
    pub v: Option<u64>,
    pub c: String,
    pub h: String,
    pub l: String,
    pub o: String,
    pub sum: Option<String>,
}

// Order book REST response
#[derive(Debug, Deserialize)]
pub struct GateOrderBookRaw {
    pub id: Option<u64>,
    pub asks: Vec<[String; 2]>,
    pub bids: Vec<[String; 2]>,
}

// Trade REST response
#[derive(Debug, Deserialize)]
pub struct GateTradeRaw {
    pub id: String,
    pub create_time: Option<String>,
    pub create_time_ms: Option<String>,
    pub currency_pair: Option<String>,
    pub side: String,
    pub amount: String,
    pub price: String,
}

// Currency pair (symbol) info
#[derive(Debug, Deserialize)]
pub struct GateCurrencyPair {
    pub id: String,
    pub base: String,
    pub quote: String,
    pub trade_status: Option<String>,
}

// WebSocket message envelope
#[derive(Debug, Deserialize)]
pub struct GateWsMessage {
    pub channel: Option<String>,
    pub event: Option<String>,
    pub result: Option<serde_json::Value>,
}

// WS candlestick update
#[derive(Debug, Deserialize)]
pub struct GateWsCandle {
    pub t: String,
    pub v: Option<String>,
    pub c: String,
    pub h: String,
    pub l: String,
    pub o: String,
    pub n: Option<String>,
}

// WS trade update
#[derive(Debug, Deserialize)]
pub struct GateWsTrade {
    pub id: Option<u64>,
    pub create_time: Option<f64>,
    pub create_time_ms: Option<String>,
    pub currency_pair: Option<String>,
    pub side: String,
    pub amount: String,
    pub price: String,
}

// WS order book update
#[derive(Debug, Deserialize)]
pub struct GateWsDepth {
    pub t: Option<u64>,
    pub s: Option<String>,
    pub asks: Vec<[String; 2]>,
    pub bids: Vec<[String; 2]>,
    #[serde(rename = "lastUpdateId")]
    pub last_update_id: Option<u64>,
}

// Server time response
#[derive(Debug, Deserialize)]
pub struct GateServerTime {
    pub server_time: Option<u64>,
}
