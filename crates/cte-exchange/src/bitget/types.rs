use serde::Deserialize;

// REST response wrapper
#[derive(Debug, Deserialize)]
pub struct BitgetResponse<T> {
    pub code: String,
    pub msg: Option<String>,
    pub data: T,
}

// Kline: array of string arrays ["ts","open","high","low","close","volume","quoteVolume"]
pub type BitgetKlineRaw = Vec<String>;

// Order book REST response
#[derive(Debug, Deserialize)]
pub struct BitgetOrderBookRaw {
    pub asks: Vec<[String; 2]>,
    pub bids: Vec<[String; 2]>,
    pub ts: Option<String>,
}

// Trade REST response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitgetTradeRaw {
    pub trade_id: String,
    pub price: String,
    pub size: String,
    pub side: String,
    pub ts: String,
}

// Ticker (used for symbols listing)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitgetTicker {
    pub symbol: String,
    #[serde(default)]
    pub base_coin: Option<String>,
    #[serde(default)]
    pub quote_coin: Option<String>,
    pub last_pr: Option<String>,
}

// Spot symbol info
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitgetSpotSymbol {
    pub symbol: String,
    pub base_coin: String,
    pub quote_coin: String,
    pub status: String,
}

// WebSocket message envelope
#[derive(Debug, Deserialize)]
pub struct BitgetWsMessage {
    pub action: Option<String>,
    pub arg: Option<BitgetWsArg>,
    pub data: Option<serde_json::Value>,
    pub event: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitgetWsArg {
    pub inst_type: Option<String>,
    pub channel: String,
    pub inst_id: Option<String>,
}

// WS Trade data
#[derive(Debug, Deserialize)]
pub struct BitgetWsTrade {
    pub ts: String,
    pub px: String,
    pub sz: String,
    pub side: String,
}

// WS Depth data
#[derive(Debug, Deserialize)]
pub struct BitgetWsDepth {
    pub asks: Vec<[String; 2]>,
    pub bids: Vec<[String; 2]>,
    pub ts: Option<String>,
}
