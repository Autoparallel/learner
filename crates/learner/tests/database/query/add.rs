// use crate::*;

use add::Add;
use learner::database::*;

use super::setup_test_db;
use crate::{create_test_paper, traced_test, TestResult};

#[traced_test]
#[test]
fn test_add_paper() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db();
  let paper = create_test_paper();

  let paper_id = Add::new(paper).execute(&mut db)?;
  assert!(paper_id > 0);
  Ok(())
}

#[traced_test]
#[test]
fn test_add_paper_twice() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db();
  let paper = create_test_paper();

  let paper_id = Add::new(paper.clone()).execute(&mut db)?;
  assert!(paper_id > 0);

  assert!(Add::new(paper).execute(&mut db).is_err());
  Ok(())
}
