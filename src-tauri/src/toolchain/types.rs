use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatus {
    pub name: String,
    pub binary: String,
    pub found: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub required: bool,
}
