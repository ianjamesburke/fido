use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VoteDirection {
    Up,
    Down,
}

impl VoteDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            VoteDirection::Up => "up",
            VoteDirection::Down => "down",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "up" => Some(VoteDirection::Up),
            "down" => Some(VoteDirection::Down),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ColorScheme {
    #[default]
    Default,
    Dark,
    Light,
    Solarized,
}

impl ColorScheme {
    pub fn as_str(&self) -> &'static str {
        match self {
            ColorScheme::Default => "Default",
            ColorScheme::Dark => "Dark",
            ColorScheme::Light => "Light",
            ColorScheme::Solarized => "Solarized",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Default" => Some(ColorScheme::Default),
            "Dark" => Some(ColorScheme::Dark),
            "Light" => Some(ColorScheme::Light),
            "Solarized" => Some(ColorScheme::Solarized),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SortOrder {
    #[default]
    Newest,
    Popular,
    Controversial,
}

impl SortOrder {
    pub fn as_str(&self) -> &'static str {
        match self {
            SortOrder::Newest => "Newest",
            SortOrder::Popular => "Popular",
            SortOrder::Controversial => "Controversial",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Newest" => Some(SortOrder::Newest),
            "Popular" => Some(SortOrder::Popular),
            "Controversial" => Some(SortOrder::Controversial),
            _ => None,
        }
    }
}
