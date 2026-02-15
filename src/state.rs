use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// RFD lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Prediscussion,
    Ideation,
    Discussion,
    Published,
    Committed,
    Abandoned,
}

impl State {
    pub const ALL: &'static [State] = &[
        State::Prediscussion,
        State::Ideation,
        State::Discussion,
        State::Published,
        State::Committed,
        State::Abandoned,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            State::Prediscussion => "prediscussion",
            State::Ideation => "ideation",
            State::Discussion => "discussion",
            State::Published => "published",
            State::Committed => "committed",
            State::Abandoned => "abandoned",
        }
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for State {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "prediscussion" => Ok(State::Prediscussion),
            "ideation" => Ok(State::Ideation),
            "discussion" => Ok(State::Discussion),
            "published" => Ok(State::Published),
            "committed" => Ok(State::Committed),
            "abandoned" => Ok(State::Abandoned),
            other => anyhow::bail!(
                "invalid state '{}'. Valid states: {}",
                other,
                State::ALL
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

/// RFD visibility levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Public,
    Internal,
    Confidential,
}

impl Default for Visibility {
    fn default() -> Self {
        Visibility::Internal
    }
}

impl Visibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Visibility::Public => "public",
            Visibility::Internal => "internal",
            Visibility::Confidential => "confidential",
        }
    }
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Visibility {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "public" => Ok(Visibility::Public),
            "internal" => Ok(Visibility::Internal),
            "confidential" => Ok(Visibility::Confidential),
            other => anyhow::bail!("invalid visibility '{}'", other),
        }
    }
}
