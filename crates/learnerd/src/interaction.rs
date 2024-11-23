use console::{style, Emoji};
use dialoguer::{Confirm, Input};

use super::*;

pub static INFO_PREFIX: &str = "ℹ ";
pub static WORKING_PREFIX: &str = "» ";
pub static SUCCESS_PREFIX: &str = "✓ ";
pub static ERROR_PREFIX: &str = "✗ ";
pub static WARNING_PREFIX: &str = "! ";
pub static PROMPT_PREFIX: &str = "❯ "; // Changed to a nicer prompt character
pub static ITEM_PREFIX: &str = "├─";
pub static LAST_ITEM_PREFIX: &str = "└─";
pub static CONTINUE_PREFIX: &str = "│  ";
pub static TREE_VERT: &str = "│";
pub static TREE_BRANCH: &str = "├";
pub static TREE_LEAF: &str = "└";
pub static BULLET: &str = "•"; // Added for lists
pub static ARROW: &str = "→"; // Added for relationships/flows

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
