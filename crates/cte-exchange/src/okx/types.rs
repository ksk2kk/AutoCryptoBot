use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct OkxResponse<T> {
    pub code: String,
    pub msg: String,
    pub data: Vec<T>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkxInstrument {
    pub inst_id: String,
    pub inst_type: String,
    pub base_ccy: String,
    pub quote_ccy: String,
    pub state: String,
    pub ct_type: Option<String>,
}

pub type OkxCandleRaw = Vec<String>;

#[derive(Debug, Deserialize)]
pub struct OkxTradeRaw {
    #[serde(rename = "instId")]
    pub inst_id: String,
    #[serde(rename = "tradeId")]
    pub trade_id: String,
    pub px: String,
    pub sz: String,
    pub side: String,
    pub ts: String,
}

#[derive(Debug, Deserialize)]
pub struct OkxDepthRaw {
    pub asks: Vec<Vec<String>>,
    pub bids: Vec<Vec<String>>,
    pub ts: String,
}

#[derive(Debug, Deserialize)]
pub struct OkxWsMessage {
    #[serde(default)]
    pub arg: Option<OkxWsArg>,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub event: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OkxWsArg {
    pub channel: String,
    #[serde(rename = "instId")]
    pub inst_id: String,
}
