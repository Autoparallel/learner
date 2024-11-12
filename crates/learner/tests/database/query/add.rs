// use crate::*;

use learner::database::{Database, *};

use super::setup_test_db;
use crate::{create_test_paper, traced_test, TestResult};

#[traced_test]
#[tokio::test]
async fn test_add_paper() -> TestResult<()> {
  let (db, _dir) = setup_test_db().await;
  let paper = create_test_paper();

  let query = QueryBuilder::<Add>::new().paper(paper).build()?;
  // Save paper
  let paper_id = db.execute(query).await?;
  dbg!(paper_id);
  assert!(paper_id > 0);
  Ok(())
}
