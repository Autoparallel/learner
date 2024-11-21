use console::{style, Emoji};
use dialoguer::{Confirm, Input};

use super::*;

pub static PAPER: Emoji<'_, '_> = Emoji("📄 ", "");
pub static ERROR: Emoji<'_, '_> = Emoji("❗️ ", "");
pub static WARNING: Emoji<'_, '_> = Emoji("⚠️  ", "");
pub static SUCCESS: Emoji<'_, '_> = Emoji("✨ ", "");
pub static INFO: Emoji<'_, '_> = Emoji("ℹ️  ", "");

#[derive(Debug)]
pub enum ResponseContent {
  Paper(Paper, bool),
  Papers(Vec<Paper>),
  Success(String),
  Error(LearnerdError),
  Info(String),
}

pub trait UserInteraction {
  fn confirm(&self, message: &str) -> Result<bool>;
  fn prompt(&self, message: &str) -> Result<String>;
  fn reply(&self, content: ResponseContent) -> Result<()>;
}
