use std::{fmt::Display, str::FromStr};

use anyhow::anyhow;

use crate::inscription::Inscription;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Filter {
    Text,
    Json,
    Brc20,
    Image,
}

impl Filter {
    pub fn all() -> Vec<Self> {
        vec![Filter::Text, Filter::Json, Filter::Brc20, Filter::Image]
    }

    pub fn inscription(&self, inscription: &Inscription) -> bool {
        match self {
            Filter::Text => inscription.parsed.is_text(),
            Filter::Json => inscription.parsed.is_json(),
            Filter::Brc20 => inscription.parsed.is_brc20(),
            Filter::Image => inscription.parsed.is_image(),
        }
    }
}

impl Display for Filter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Filter::Text => f.write_str("Text")?,
            Filter::Json => f.write_str("JSON")?,
            Filter::Brc20 => f.write_str("BRC-20")?,
            Filter::Image => f.write_str("Image")?,
        }

        Ok(())
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
