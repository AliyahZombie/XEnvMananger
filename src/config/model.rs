use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProfileSource {
    Protocol,
    Preset,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvVarType {
    Secret,
    String,
    Number,
    Boolean,
    Enum,
    Path,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StoredEnvVar {
    Secret(StoredSecret),
    String { value: String },
    Number { value: i64 },
    Boolean { value: bool },
    Enum { value: String },
    Path { value: String },
}

fn default_secret_required() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "storage", rename_all = "lowercase")]
pub enum StoredSecret {
    Keyring {
        #[serde(default = "default_secret_required")]
        required: bool,

        keyring_key: String,
    },

    Plain {
        #[serde(default = "default_secret_required")]
        required: bool,

        #[serde(default)]
        value: String,
    },
}

impl StoredSecret {
    pub fn required(&self) -> bool {
        match self {
            StoredSecret::Keyring { required, .. } => *required,
            StoredSecret::Plain { required, .. } => *required,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub program: String,
    pub source: ProfileSource,

    /// RFC3339 timestamp (UTC) as string. Keep as String to avoid time crate MSRV issues.
    pub last_used: Option<String>,

    pub env_vars: BTreeMap<String, StoredEnvVar>,
}
