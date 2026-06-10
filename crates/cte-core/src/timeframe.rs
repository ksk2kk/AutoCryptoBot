use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Timeframe {
    S1,
    S10,
    M1,
    M3,
    M5,
    M15,
    M30,
    H1,
    H2,
    H4,
    H6,
    H8,
    H12,
    D1,
    D3,
    W1,
    MN1,
}

impl Timeframe {
    pub fn to_seconds(&self) -> u64 {
        match self {
            Self::S1 => 1,
            Self::S10 => 10,
            Self::M1 => 60,
            Self::M3 => 180,
            Self::M5 => 300,
            Self::M15 => 900,
            Self::M30 => 1800,
            Self::H1 => 3600,
            Self::H2 => 7200,
            Self::H4 => 14400,
            Self::H6 => 21600,
            Self::H8 => 28800,
            Self::H12 => 43200,
            Self::D1 => 86400,
            Self::D3 => 259200,
            Self::W1 => 604800,
            Self::MN1 => 2592000,
        }
    }

    pub fn to_binance_str(&self) -> &'static str {
        match self {
            Self::S1 => "1s",
            Self::S10 => "1m",
            Self::M1 => "1m",
            Self::M3 => "3m",
            Self::M5 => "5m",
            Self::M15 => "15m",
            Self::M30 => "30m",
            Self::H1 => "1h",
            Self::H2 => "2h",
            Self::H4 => "4h",
            Self::H6 => "6h",
            Self::H8 => "8h",
            Self::H12 => "12h",
            Self::D1 => "1d",
            Self::D3 => "3d",
            Self::W1 => "1w",
            Self::MN1 => "1M",
        }
    }

    pub fn to_okx_str(&self) -> &'static str {
        match self {
            Self::S1 => "1m",
            Self::S10 => "1m",
            Self::M1 => "1m",
            Self::M3 => "3m",
            Self::M5 => "5m",
            Self::M15 => "15m",
            Self::M30 => "30m",
            Self::H1 => "1H",
            Self::H2 => "2H",
            Self::H4 => "4H",
            Self::H6 => "6H",
            Self::H8 => "6H",
            Self::H12 => "12H",
            Self::D1 => "1D",
            Self::D3 => "3D",
            Self::W1 => "1W",
            Self::MN1 => "1M",
        }
    }

    pub fn to_bybit_str(&self) -> &'static str {
        match self {
            Self::S1 => "1",
            Self::S10 => "1",
            Self::M1 => "1",
            Self::M3 => "3",
            Self::M5 => "5",
            Self::M15 => "15",
            Self::M30 => "30",
            Self::H1 => "60",
            Self::H2 => "120",
            Self::H4 => "240",
            Self::H6 => "360",
            Self::H8 => "720",
            Self::H12 => "720",
            Self::D1 => "D",
            Self::D3 => "D",
            Self::W1 => "W",
            Self::MN1 => "M",
        }
    }

    pub fn to_gateio_str(&self) -> &'static str {
        match self {
            Self::S1 => "10s",
            Self::S10 => "10s",
            Self::M1 => "1m",
            Self::M3 => "5m",
            Self::M5 => "5m",
            Self::M15 => "15m",
            Self::M30 => "30m",
            Self::H1 => "1h",
            Self::H2 => "4h",
            Self::H4 => "4h",
            Self::H6 => "8h",
            Self::H8 => "8h",
            Self::H12 => "1d",
            Self::D1 => "1d",
            Self::D3 => "7d",
            Self::W1 => "7d",
            Self::MN1 => "30d",
        }
    }

    pub fn to_bitget_str(&self) -> &'static str {
        match self {
            Self::S1 => "1m",
            Self::S10 => "1m",
            Self::M1 => "1m",
            Self::M3 => "3m",
            Self::M5 => "5m",
            Self::M15 => "15m",
            Self::M30 => "30m",
            Self::H1 => "1H",
            Self::H2 => "2H",
            Self::H4 => "4H",
            Self::H6 => "6H",
            Self::H8 => "12H",
            Self::H12 => "12H",
            Self::D1 => "1D",
            Self::D3 => "3D",
            Self::W1 => "1W",
            Self::MN1 => "1M",
        }
    }
}

impl fmt::Display for Timeframe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::S1 => write!(f, "1s"),
            Self::S10 => write!(f, "10s"),
            Self::M1 => write!(f, "1m"),
            Self::M3 => write!(f, "3m"),
            Self::M5 => write!(f, "5m"),
            Self::M15 => write!(f, "15m"),
            Self::M30 => write!(f, "30m"),
            Self::H1 => write!(f, "1h"),
            Self::H2 => write!(f, "2h"),
            Self::H4 => write!(f, "4h"),
            Self::H6 => write!(f, "6h"),
            Self::H8 => write!(f, "8h"),
            Self::H12 => write!(f, "12h"),
            Self::D1 => write!(f, "1d"),
            Self::D3 => write!(f, "3d"),
            Self::W1 => write!(f, "1w"),
            Self::MN1 => write!(f, "1M"),
        }
    }
}

impl std::str::FromStr for Timeframe {
    type Err = crate::error::CteError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "1s" | "s1" => Ok(Self::S1),
            "10s" | "s10" => Ok(Self::S10),
            "1m" | "m1" => Ok(Self::M1),
            "3m" | "m3" => Ok(Self::M3),
            "5m" | "m5" => Ok(Self::M5),
            "15m" | "m15" => Ok(Self::M15),
            "30m" | "m30" => Ok(Self::M30),
            "1h" | "h1" => Ok(Self::H1),
            "2h" | "h2" => Ok(Self::H2),
            "4h" | "h4" => Ok(Self::H4),
            "6h" | "h6" => Ok(Self::H6),
            "8h" | "h8" => Ok(Self::H8),
            "12h" | "h12" => Ok(Self::H12),
            "1d" | "d1" => Ok(Self::D1),
            "3d" | "d3" => Ok(Self::D3),
            "1w" | "w1" => Ok(Self::W1),
            "1mn" | "mn1" | "1m_month" | "month" => Ok(Self::MN1),
            _ => Err(crate::error::CteError::Config(format!(
                "Unknown timeframe: {s}. Valid: 1s,10s,1m,3m,5m,15m,30m,1h,2h,4h,6h,8h,12h,1d,3d,1w,1M"
            ))),
        }
    }
}
