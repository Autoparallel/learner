// use crate::*;

use add::Add;
use learner::database::*;

use super::setup_test_db;
use crate::{create_test_paper, traced_test, TestResult};

#[traced_test]
#[tokio::test]
async fn test_add_paper() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db().await;
  let paper = create_test_paper();

  let papers = Add::paper(&paper).execute(&mut db).await?;
  assert!(papers[0].id() > 0);
  Ok(())
}

#[traced_test]
#[tokio::test]
async fn test_add_paper_twice() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db().await;
  let paper = create_test_paper();

  let papers = Add::paper(&paper).execute(&mut db).await?;
  assert!(papers[0].id() > 0);

  assert!(Add::paper(&paper).execute(&mut db).await.is_err());
  Ok(())
}
