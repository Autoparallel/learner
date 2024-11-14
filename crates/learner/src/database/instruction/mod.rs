use super::*;

pub mod add;
pub mod query;
pub mod remove;

use rusqlite::params;

pub trait DatabaseInstruction {
  type Output;

  // Take &mut reference to avoid taking ownership and allow multiple operations
  fn execute(&self, db: &mut Database) -> Result<Self::Output>;
}
