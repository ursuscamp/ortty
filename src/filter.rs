use std::str::FromStr;

use anyhow::anyhow;

use crate::inscription::Inscription;

#[derive(Debug, Copy, Clone)]
pub enum Filter {
    Text,
    Json,
    Brc20,
    Image,
}

impl Filter {
    pub fn inscription(&self, inscription: &Inscription) -> bool {
        let text = std::str::from_utf8(&inscription.data).ok();
        let json: Option<serde_json::Value> = text.map(|s| serde_json::from_str(s).ok()).flatten();
        let brc20 = json
            .as_ref()
            .map(|v| v.get("p").map(|s| s == "brc-20"))
            .flatten()
            .unwrap_or_default();
        match self {
            Filter::Text => text.is_some(),
            Filter::Json => json.is_some(),
            Filter::Brc20 => brc20,
            Filter::Image => inscription.mime.type_() == "image",
        }
    }
}

impl FromStr for Filter {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let filter = match s.to_lowercase().as_ref() {
            "text" => Self::Text,
            "json" => Self::Json,
            "brc20" | "brc-20" => Self::Brc20,
            "image" => Self::Image,
            _ => return Err(anyhow!("Unknown filter type")),
        };
        Ok(filter)
    }
}
