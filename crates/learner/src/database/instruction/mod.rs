use super::*;

pub mod add;
pub mod query;
pub mod remove;

use rusqlite::params;

#[async_trait::async_trait]
pub trait DatabaseInstruction {
  type Output;

  // Take &mut reference to avoid taking ownership and allow multiple operations
  async fn execute(&self, db: &mut Database) -> Result<Self::Output>;
}
