use std::marker::PhantomData;

use super::*;

pub mod add;

// Type alias for our closure
pub type QueryFn<T> = Box<dyn FnOnce(&mut rusqlite::Connection) -> Result<T> + Send>;

pub mod state {
  pub struct Get;
  pub struct Search;
  pub struct Add;
  pub struct Remove;
  pub struct List;
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

impl<S> QueryBuilder<S> {
  pub fn new() -> Self {
    Self {
      state:        PhantomData,
      source:       None,
      identifier:   None,
      paper:        None,
      search_terms: None,
      order_by:     None,
      descending:   false,
    }
  }
}
