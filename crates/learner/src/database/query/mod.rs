use super::*;

pub mod add;
pub mod search;

use rusqlite::params;

// We could make this even more specific about what we're doing
pub trait DatabaseStatement {
  type Output;

  // Take &mut reference to avoid taking ownership and allow multiple operations
  fn execute(&self, db: &mut Database) -> Result<Self::Output>;
}

// #[derive(Debug, Clone, Copy)]
// pub enum PaperOrderField {
//   Title,
//   Date,
//   Source,
// }

// pub struct QueryBuilder<S> {
//   pub(crate) state:        PhantomData<S>,
//   pub(crate) source:       Option<Source>,
//   pub(crate) identifier:   Option<String>,
//   pub(crate) paper:        Option<Paper>,
//   pub(crate) search_terms: Option<String>,
//   pub(crate) order_by:     Option<PaperOrderField>,
//   pub(crate) descending:   bool,
// }
