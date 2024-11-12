use chrono::{TimeZone, Utc};
use learner::{
  database::{add::Add, search::Search, *},
  paper::{Author, Paper, Source},
};

use super::setup_test_db;
use crate::{create_second_test_paper, create_test_paper, traced_test, TestResult};

#[test]
#[traced_test]
fn test_search_by_text() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db();

  // First add a test paper
  let paper = create_test_paper();
  Add::new(paper).execute(&mut db)?;

  // Search for it
  let results = Search::text("test paper").execute(&mut db)?;

  assert_eq!(results.len(), 1);
  assert_eq!(results[0].title, "Test Paper");
  Ok(())
}

#[test]
fn test_search_by_author() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db();

  // Add test paper
  let paper = create_test_paper();
  Add::new(paper).execute(&mut db)?;

  // Search by author
  let results = Search::by_author("John Doe").execute(&mut db)?;

  assert_eq!(results.len(), 1);
  assert_eq!(results[0].authors[0].name, "John Doe");
  Ok(())
}

#[traced_test]
#[test]
fn test_search_by_text_case_insensitive() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db();
  let paper = create_test_paper();
  Add::new(paper).execute(&mut db)?;

  // Search with different case
  let results = Search::text("TEST PAPER").execute(&mut db)?;
  assert_eq!(results.len(), 1);
  assert_eq!(results[0].title, "Test Paper");

  Ok(())
}

#[traced_test]
#[test]
fn test_search_by_source() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db();

  let paper = create_test_paper();
  Add::new(paper).execute(&mut db)?;

  let results = Search::by_source(Source::Arxiv, "2301.00000").execute(&mut db)?;
  assert_eq!(results.len(), 1);
  assert_eq!(results[0].source_identifier, "2301.00000");

  // Search for non-existent paper
  let results = Search::by_source(Source::Arxiv, "nonexistent").execute(&mut db)?;
  assert_eq!(results.len(), 0);

  Ok(())
}

#[traced_test]
#[test]
fn test_search_ordering() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db();

  let paper1 = create_test_paper();
  let paper2 = create_second_test_paper();

  Add::new(paper1).execute(&mut db)?;
  Add::new(paper2).execute(&mut db)?;

  // Test ascending order by date
  let results = Search::text("paper").order_by("publication_date").execute(&mut db)?;
  assert_eq!(results.len(), 2);
  assert_eq!(results[0].title, "Test Paper");
  assert_eq!(results[1].title, "Test Paper: Two");

  // Test descending order by date
  let results = Search::text("paper").order_by("publication_date").descending().execute(&mut db)?;
  assert_eq!(results[0].title, "Test Paper: Two");
  assert_eq!(results[1].title, "Test Paper");

  // Test ordering by title
  let results = Search::text("paper").order_by("title").execute(&mut db)?;
  assert_eq!(results[0].title, "Test Paper");
  assert_eq!(results[1].title, "Test Paper: Two");

  Ok(())
}

#[traced_test]
#[test]
fn test_search_by_partial_author_name() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db();

  // Create paper with multiple authors
  let mut paper = create_test_paper();
  paper.authors =
    vec![Author { name: "John Smith".to_string(), affiliation: None, email: None }, Author {
      name:        "Jane Smith".to_string(),
      affiliation: None,
      email:       None,
    }];
  Add::new(paper).execute(&mut db)?;

  // Test partial name matches
  let results = Search::by_author("Smith").execute(&mut db)?;
  assert_eq!(results.len(), 1);
  assert_eq!(results[0].authors.len(), 2);

  let results = Search::by_author("John").execute(&mut db)?;
  assert_eq!(results.len(), 1);

  // Test case insensitivity
  let results = Search::by_author("JOHN").execute(&mut db)?;
  assert_eq!(results.len(), 1);

  // Test no matches
  let results = Search::by_author("Wilson").execute(&mut db)?;
  assert_eq!(results.len(), 0);

  Ok(())
}

#[traced_test]
#[test]
fn test_search_multiple_results() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db();

  let paper1 = create_test_paper();
  let paper2 = create_second_test_paper();

  Add::new(paper1).execute(&mut db)?;
  Add::new(paper2).execute(&mut db)?;

  // Search for "quantum" should return 2 papers
  let results = Search::text("test").execute(&mut db)?;
  assert_eq!(results.len(), 2);
  assert!(results.iter().all(|p| p.title.to_lowercase().contains("test")));

  // Search for "computing" should return all 3 papers
  let results = Search::text("two").execute(&mut db)?;
  assert_eq!(results.len(), 1);

  Ok(())
}

#[traced_test]
#[test]
fn test_invalid_order_field() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db();
  Add::new(create_test_paper()).execute(&mut db)?;

  // Test with invalid order field
  let result = Search::text("test").order_by("nonexistent_field").execute(&mut db);
  assert!(result.is_err());

  Ok(())
}
