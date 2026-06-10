use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BinanceKlineRaw(
    pub i64,    // 0: open time
    pub String, // 1: open
    pub String, // 2: high
    pub String, // 3: low
    pub String, // 4: close
    pub String, // 5: volume
    pub i64,    // 6: close time
    pub String, // 7: quote asset volume
    pub u64,    // 8: number of trades
    pub String, // 9: taker buy base volume
    pub String, // 10: taker buy quote volume
    pub String, // 11: ignore
);

#[derive(Debug, Deserialize)]
pub struct BinanceWsKlineEvent {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: i64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "k")]
    pub kline: BinanceWsKline,
}

#[derive(Debug, Deserialize)]
pub struct BinanceWsKline {
    #[serde(rename = "t")]
    pub open_time: i64,
    #[serde(rename = "T")]
    pub close_time: i64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "i")]
    pub interval: String,
    #[serde(rename = "o")]
    pub open: String,
    #[serde(rename = "c")]
    pub close: String,
    #[serde(rename = "h")]
    pub high: String,
    #[serde(rename = "l")]
    pub low: String,
    #[serde(rename = "v")]
    pub volume: String,
    #[serde(rename = "n")]
    pub trades: u64,
    #[serde(rename = "x")]
    pub is_closed: bool,
    #[serde(rename = "q")]
    pub quote_volume: String,
}

#[derive(Debug, Deserialize)]
pub struct BinanceWsTradeEvent {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: i64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "t")]
    pub trade_id: u64,
    #[serde(rename = "p")]
    pub price: String,
    #[serde(rename = "q")]
    pub quantity: String,
    #[serde(rename = "T")]
    pub trade_time: i64,
    #[serde(rename = "m")]
    pub is_buyer_maker: bool,
}

#[derive(Debug, Deserialize)]
pub struct BinanceDepthEvent {
    #[serde(rename = "lastUpdateId")]
    pub last_update_id: u64,
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
}

#[derive(Debug, Deserialize)]
pub struct BinanceWsDepthEvent {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: i64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "lastUpdateId")]
    pub last_update_id: Option<u64>,
    #[serde(rename = "u")]
    pub final_update_id: Option<u64>,
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
}

#[derive(Debug, Deserialize)]
pub struct BinanceExchangeInfo {
    pub symbols: Vec<BinanceSymbolInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceSymbolInfo {
    pub symbol: String,
    pub base_asset: String,
    pub quote_asset: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct BinanceFuturesExchangeInfo {
    pub symbols: Vec<BinanceFuturesSymbolInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceFuturesSymbolInfo {
    pub symbol: String,
    pub base_asset: String,
    pub quote_asset: String,
    pub status: String,
    pub contract_type: String,
}

#[derive(Debug, Deserialize)]
pub struct BinanceTradeRaw {
    pub id: u64,
    pub price: String,
    pub qty: String,
    pub time: i64,
    #[serde(rename = "isBuyerMaker")]
    pub is_buyer_maker: bool,
}
