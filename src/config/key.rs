use serde::{Deserialize, Serialize};
use std::{fmt::Display, ops::Deref};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(transparent)]
pub struct GeminiKey {
    pub inner: String,
}

impl Deref for GeminiKey {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl GeminiKey {
    pub fn ellipse(&self) -> String {
        let len = self.inner.len();
        if len > 10 {
            format!("{}...", &self.inner[..10])
        } else {
            self.inner.to_owned()
        }
    }
}

impl Display for GeminiKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}
