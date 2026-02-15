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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Public,
    #[default]
    Internal,
    Confidential,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_roundtrip_all() {
        for state in State::ALL {
            let s = state.as_str();
            let parsed: State = s.parse().unwrap();
            assert_eq!(*state, parsed);
            assert_eq!(state.to_string(), s);
        }
    }

    #[test]
    fn test_state_from_str_case_insensitive() {
        assert_eq!(State::from_str("Discussion").unwrap(), State::Discussion);
        assert_eq!(State::from_str("PUBLISHED").unwrap(), State::Published);
        assert_eq!(State::from_str("  ideation  ").unwrap(), State::Ideation);
    }

    #[test]
    fn test_state_from_str_invalid() {
        let err = State::from_str("bogus").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("invalid state 'bogus'"));
        assert!(msg.contains("prediscussion"));
    }

    #[test]
    fn test_visibility_roundtrip() {
        let cases = [
            (Visibility::Public, "public"),
            (Visibility::Internal, "internal"),
            (Visibility::Confidential, "confidential"),
        ];
        for (vis, expected) in cases {
            assert_eq!(vis.as_str(), expected);
            assert_eq!(vis.to_string(), expected);
            assert_eq!(Visibility::from_str(expected).unwrap(), vis);
        }
    }

    #[test]
    fn test_visibility_default_is_internal() {
        assert_eq!(Visibility::default(), Visibility::Internal);
    }

    #[test]
    fn test_visibility_from_str_invalid() {
        assert!(Visibility::from_str("secret").is_err());
    }
}
