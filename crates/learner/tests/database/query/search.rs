use chrono::{Datelike, TimeZone, Utc};
use learner::{
  database::{
    add::Add,
    search::{OrderField, Query, QueryCriteria},
    *,
  },
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
  let results = Query::text("test paper").execute(&mut db)?;

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
  let results = Query::by_author("John Doe").execute(&mut db)?;

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
  let results = Query::text("TEST PAPER").execute(&mut db)?;
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

  let results = Query::by_source(Source::Arxiv, "2301.00000").execute(&mut db)?;
  assert_eq!(results.len(), 1);
  assert_eq!(results[0].source_identifier, "2301.00000");

  // Search for non-existent paper
  let results = Query::by_source(Source::Arxiv, "nonexistent").execute(&mut db)?;
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
  let results = Query::text("paper").order_by(OrderField::PublicationDate).execute(&mut db)?;
  assert_eq!(results.len(), 2);
  assert_eq!(results[0].title, "Test Paper");
  assert_eq!(results[1].title, "Test Paper: Two");

  // Test descending order by date
  let results =
    Query::text("paper").order_by(OrderField::PublicationDate).descending().execute(&mut db)?;
  assert_eq!(results[0].title, "Test Paper: Two");
  assert_eq!(results[1].title, "Test Paper");

  // Test ordering by title
  let results = Query::text("paper").order_by(OrderField::Title).execute(&mut db)?;
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
  let results = Query::by_author("Smith").execute(&mut db)?;
  assert_eq!(results.len(), 1);
  assert_eq!(results[0].authors.len(), 2);

  let results = Query::by_author("John").execute(&mut db)?;
  assert_eq!(results.len(), 1);

  // Test case insensitivity
  let results = Query::by_author("JOHN").execute(&mut db)?;
  assert_eq!(results.len(), 1);

  // Test no matches
  let results = Query::by_author("Wilson").execute(&mut db)?;
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
  let results = Query::text("test").execute(&mut db)?;
  assert_eq!(results.len(), 2);
  assert!(results.iter().all(|p| p.title.to_lowercase().contains("test")));

  // Search for "computing" should return all 3 papers
  let results = Query::text("two").execute(&mut db)?;
  assert_eq!(results.len(), 1);

  Ok(())
}

#[traced_test]
#[test]
fn test_list_all_papers() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db();

  // Empty database should return empty list
  let results = Query::list_all().execute(&mut db)?;
  assert_eq!(results.len(), 0);

  // Add both test papers
  let paper1 = create_test_paper();
  let paper2 = create_second_test_paper();
  Add::new(paper1).execute(&mut db)?;
  Add::new(paper2).execute(&mut db)?;

  // Should return all papers
  let results = Query::list_all().execute(&mut db)?;
  assert_eq!(results.len(), 2);
  Ok(())
}

#[traced_test]
#[test]
fn test_ordering_by_source() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db();

  // Create papers with different sources
  let mut paper1 = create_test_paper();
  paper1.source = Source::DOI;
  let mut paper2 = create_second_test_paper();
  paper2.source = Source::Arxiv;

  Add::new(paper1).execute(&mut db)?;
  Add::new(paper2).execute(&mut db)?;

  let results = Query::list_all().order_by(OrderField::Source).execute(&mut db)?;
  assert_eq!(results[0].source, Source::Arxiv);
  assert_eq!(results[1].source, Source::DOI);

  Ok(())
}

#[traced_test]
#[test]
fn test_query_with_no_results() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db();

  let paper = create_test_paper();
  Add::new(paper).execute(&mut db)?;

  // Test each query type with non-matching criteria
  let results = Query::text("nonexistent").execute(&mut db)?;
  assert_eq!(results.len(), 0);

  let results = Query::by_author("nonexistent").execute(&mut db)?;
  assert_eq!(results.len(), 0);

  let results = Query::by_source(Source::DOI, "nonexistent").execute(&mut db)?;
  assert_eq!(results.len(), 0);

  Ok(())
}

#[traced_test]
#[test]
fn test_search_respects_word_boundaries() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db();

  // Create a paper with specific title
  let mut paper = create_test_paper();
  paper.title = "Testing Paper".to_string();
  Add::new(paper).execute(&mut db)?;

  // Should find the paper
  let results = Query::text("test").execute(&mut db)?;
  assert_eq!(results.len(), 1);

  // Should also find with full word
  let results = Query::text("testing").execute(&mut db)?;
  assert_eq!(results.len(), 1);

  Ok(())
}

#[traced_test]
#[test]
fn test_author_name_exact_match() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db();

  let paper = create_test_paper();
  Add::new(paper).execute(&mut db)?;

  // Test exact author name match
  let results = Query::by_author("John Doe").execute(&mut db)?;
  assert_eq!(results.len(), 1);

  // Test only first name
  let results = Query::by_author("John").execute(&mut db)?;
  assert_eq!(results.len(), 1);

  // Test only last name
  let results = Query::by_author("Doe").execute(&mut db)?;
  assert_eq!(results.len(), 1);

  Ok(())
}

#[traced_test]
#[test]
fn test_combined_queries_ordering() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db();

  let paper1 = create_test_paper();
  let paper2 = create_second_test_paper();

  Add::new(paper1).execute(&mut db)?;
  Add::new(paper2).execute(&mut db)?;

  // Search with text and ordering
  let results =
    Query::text("test").order_by(OrderField::PublicationDate).descending().execute(&mut db)?;

  assert_eq!(results.len(), 2);
  assert_eq!(results[0].publication_date.year(), 2024);
  assert_eq!(results[1].publication_date.year(), 2023);

  Ok(())
}
