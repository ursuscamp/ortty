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
        match self {
            Filter::Text => inscription.parsed.is_text(),
            Filter::Json => inscription.parsed.is_json(),
            Filter::Brc20 => inscription.parsed.is_brc20(),
            Filter::Image => inscription.parsed.is_image(),
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
