//! Agent manager - placeholder implementation

use crate::error::AppResult;

pub struct AgentManager {}

impl AgentManager {
    pub async fn new() -> AppResult<Self> {
        Ok(Self {})
    }
}
