//! Key binding types.

use serde::{Deserialize, Serialize};

/// Key modifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Modifier {
    #[default]
    None,
    Ctrl,
    Alt,
    Shift,
}

/// A key combination
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyBinding {
    /// The key character or name
    pub key: String,

    /// Modifier key
    #[serde(default)]
    pub modifier: Modifier,
}

impl KeyBinding {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            modifier: Modifier::None,
        }
    }

    pub fn with_ctrl(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            modifier: Modifier::Ctrl,
        }
    }

    pub fn with_shift(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            modifier: Modifier::Shift,
        }
    }
}
