use serde::Deserialize;

// REST response wrapper
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BybitResponse<T> {
    pub ret_code: i32,
    pub ret_msg: Option<String>,
    pub result: T,
}

// Kline REST response
#[derive(Debug, Deserialize)]
pub struct BybitKlineResult {
    pub list: Vec<Vec<String>>,
}

// Order book REST response
#[derive(Debug, Deserialize)]
pub struct BybitOrderBookResult {
    pub b: Vec<[String; 2]>,
    pub a: Vec<[String; 2]>,
    pub ts: Option<u64>,
    pub u: Option<u64>,
}

// Trade REST response
#[derive(Debug, Deserialize)]
pub struct BybitTradeResult {
    pub list: Vec<BybitTradeRaw>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BybitTradeRaw {
    #[serde(rename = "execId")]
    pub exec_id: String,
    pub symbol: String,
    pub price: String,
    pub size: String,
    pub side: String,
    pub time: String,
}

// Instruments info REST response
#[derive(Debug, Deserialize)]
pub struct BybitInstrumentsResult {
    pub list: Vec<BybitInstrumentInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BybitInstrumentInfo {
    pub symbol: String,
    pub base_coin: Option<String>,
    pub quote_coin: Option<String>,
    pub status: String,
}

// WebSocket message envelope
#[derive(Debug, Deserialize)]
pub struct BybitWsMessage {
    pub topic: Option<String>,
    pub data: Option<serde_json::Value>,
    #[serde(rename = "type")]
    pub msg_type: Option<String>,
    pub op: Option<String>,
}

// WS Kline data
#[derive(Debug, Deserialize)]
pub struct BybitWsKline {
    pub start: u64,
    pub end: u64,
    pub interval: String,
    pub open: String,
    pub close: String,
    pub high: String,
    pub low: String,
    pub volume: String,
    pub turnover: String,
    pub confirm: bool,
}

// WS Trade data
#[derive(Debug, Deserialize)]
pub struct BybitWsTrade {
    #[serde(rename = "T")]
    pub timestamp: u64,
    pub s: String,
    #[serde(rename = "S")]
    pub side: String,
    pub v: String,
    pub p: String,
    pub i: Option<String>,
}

// WS Depth data
#[derive(Debug, Deserialize)]
pub struct BybitWsDepth {
    pub s: String,
    pub b: Vec<[String; 2]>,
    pub a: Vec<[String; 2]>,
    pub u: Option<u64>,
    pub seq: Option<u64>,
}
