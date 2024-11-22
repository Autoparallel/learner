use console::{style, Emoji};
use dialoguer::{Confirm, Input};

use super::*;

pub static INFO_PREFIX: &str = "│ "; // Information/status
pub static SUCCESS_PREFIX: &str = "✓ "; // Success/completion
pub static ERROR_PREFIX: &str = "✗ "; // Error/failure
pub static WARNING_PREFIX: &str = "! "; // Warning/caution
pub static PROMPT_PREFIX: &str = "> "; // User prompt
pub static ITEM_PREFIX: &str = "├─"; // List item
pub static LAST_ITEM_PREFIX: &str = "└─"; // Last list item
pub static CONTINUE_PREFIX: &str = "│  "; // Continuation line
pub static TREE_VERT: &str = "│";
pub static TREE_BRANCH: &str = "├";
pub static TREE_LEAF: &str = "└";

#[derive(Debug)]
pub enum ResponseContent<'a> {
  Paper(&'a Paper),
  Papers(&'a [Paper]),
  Success(&'a str),
  Error(LearnerdError),
  Info(&'a str),
}

pub trait UserInteraction {
  fn confirm(&self, message: &str) -> Result<bool>;
  fn prompt(&self, message: &str) -> Result<String>;
  fn reply(&self, content: ResponseContent) -> Result<()>;
}
