use std::marker::PhantomData;

use super::*;

pub mod add;

// Type alias for our closure
pub type StatementFunction<T> = Box<dyn FnOnce(&mut rusqlite::Connection) -> Result<T> + Send>;

pub trait Statement<T> {
  fn build(self) -> Result<StatementFunction<T>>;
}

#[derive(Debug, Clone, Copy)]
pub enum PaperOrderField {
  Title,
  Date,
  Source,
}

pub struct QueryBuilder<S> {
  pub(crate) state:        PhantomData<S>,
  pub(crate) source:       Option<Source>,
  pub(crate) identifier:   Option<String>,
  pub(crate) paper:        Option<Paper>,
  pub(crate) search_terms: Option<String>,
  pub(crate) order_by:     Option<PaperOrderField>,
  pub(crate) descending:   bool,
}
