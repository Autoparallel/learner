use chrono::{TimeZone, Utc};
use learner::{
  database::{
    add::Add,
    query::{OrderField, Query},
    remove::*,
    *,
  },
  paper::{Author, Source},
};

use super::setup_test_db;
use crate::{create_second_test_paper, create_test_paper, traced_test, TestResult};

/// Basic removal functionality tests
mod basic_operations {
  use super::*;

  #[test]
  fn test_remove_existing_paper() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db();

    let paper = create_test_paper();
    let source = paper.source.clone();
    let id = paper.source_identifier.clone();
    Add::new(paper.clone()).execute(&mut db)?;

    let removed_papers = Remove::by_source(source.clone(), id.clone()).execute(&mut db)?;

    assert_eq!(removed_papers.len(), 1);
    assert_eq!(removed_papers[0].title, paper.title);
    assert_eq!(removed_papers[0].authors.len(), paper.authors.len());

    let results = Query::by_source(source, id).execute(&mut db)?;
    assert_eq!(results.len(), 0);

    Ok(())
  }

  #[test]
  fn test_remove_nonexistent_paper() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db();

    let removed = Remove::by_source(Source::Arxiv, "nonexistent").execute(&mut db)?;
    assert!(removed.is_empty());

    Ok(())
  }

  #[test]
  fn test_remove_cascades_to_authors() -> TestResult<()> {
    todo!()
    // let (mut db, _dir) = setup_test_db();

    // let paper = create_test_paper();
    // Add::new(paper).execute(&mut db)?;

    // Remove::from_query(Query::text("test")).execute(&mut db)?;

    // let count: i64 =
    //   db.conn.prepare("SELECT COUNT(*) FROM authors")?.query_row([], |row| row.get(0))?;

    // assert_eq!(count, 0);
    // Ok(())
  }
}

/// Dry run functionality tests
mod dry_run {
  use super::*;

  #[test]
  fn test_dry_run_basic() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db();

    let paper = create_test_paper();
    let source = paper.source.clone();
    let id = paper.source_identifier.clone();
    Add::new(paper.clone()).execute(&mut db)?;

    let would_remove = Remove::by_source(source.clone(), id.clone()).dry_run().execute(&mut db)?;

    assert_eq!(would_remove.len(), 1);
    assert_eq!(would_remove[0].title, paper.title);

    let results = Query::by_source(source, id).execute(&mut db)?;
    assert_eq!(results.len(), 1);

    Ok(())
  }

  #[test]
  fn test_dry_run_returns_complete_paper() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db();

    let paper = create_test_paper();
    Add::new(paper.clone()).execute(&mut db)?;

    let would_remove = Remove::from_query(Query::text("test")).dry_run().execute(&mut db)?;

    assert_eq!(would_remove.len(), 1);
    let removed = &would_remove[0];

    // Verify all fields
    assert_eq!(removed.title, paper.title);
    assert_eq!(removed.abstract_text, paper.abstract_text);
    assert_eq!(removed.publication_date, paper.publication_date);
    assert_eq!(removed.source, paper.source);
    assert_eq!(removed.source_identifier, paper.source_identifier);
    assert_eq!(removed.pdf_url, paper.pdf_url);
    assert_eq!(removed.doi, paper.doi);
    assert_eq!(removed.authors.len(), paper.authors.len());

    for (removed_author, original_author) in removed.authors.iter().zip(paper.authors.iter()) {
      assert_eq!(removed_author.name, original_author.name);
      assert_eq!(removed_author.affiliation, original_author.affiliation);
      assert_eq!(removed_author.email, original_author.email);
    }

    Ok(())
  }
}

/// Tests for Query-based removal features
mod query_based_removal {
  use super::*;

  #[test]
  fn test_remove_by_text_search() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db();

    Add::new(create_test_paper()).execute(&mut db)?;
    Add::new(create_second_test_paper()).execute(&mut db)?;

    let removed = Remove::from_query(Query::text("two")).execute(&mut db)?;
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].title, "Test Paper: Two");

    Ok(())
  }

  #[test]
  fn test_remove_by_author() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db();

    Add::new(create_test_paper()).execute(&mut db)?;
    Add::new(create_second_test_paper()).execute(&mut db)?;

    let removed = Remove::from_query(Query::by_author("John Doe")).execute(&mut db)?;
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].authors[0].name, "John Doe");

    Ok(())
  }

  #[test]
  fn test_remove_with_ordering() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db();

    Add::new(create_test_paper()).execute(&mut db)?;
    Add::new(create_second_test_paper()).execute(&mut db)?;

    let removed =
      Remove::from_query(Query::text("test").order_by(OrderField::PublicationDate).descending())
        .execute(&mut db)?;

    assert_eq!(removed.len(), 2);
    assert_eq!(removed[0].title, "Test Paper: Two"); // More recent
    assert_eq!(removed[1].title, "Test Paper");

    Ok(())
  }
}

/// Recovery and data integrity tests
mod recovery {
  use super::*;

  #[test]
  fn test_remove_papers_can_be_readded() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db();

    let paper = create_test_paper();
    Add::new(paper.clone()).execute(&mut db)?;

    let removed_papers = Remove::from_query(Query::text("test")).execute(&mut db)?;
    assert_eq!(removed_papers.len(), 1);

    Add::new(removed_papers[0].clone()).execute(&mut db)?;

    let results = Query::text("test").execute(&mut db)?;
    assert_eq!(results.len(), 1);

    Ok(())
  }

  #[test]
  fn test_bulk_remove_and_readd() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db();

    // Add multiple papers
    Add::new(create_test_paper()).execute(&mut db)?;
    Add::new(create_second_test_paper()).execute(&mut db)?;

    // Remove all test papers
    let removed = Remove::from_query(Query::text("test")).execute(&mut db)?;
    assert_eq!(removed.len(), 2);

    // Readd them all
    for paper in removed {
      Add::new(paper).execute(&mut db)?;
    }

    let results = Query::text("test").execute(&mut db)?;
    assert_eq!(results.len(), 2);

    Ok(())
  }
}

#[test]
fn test_remove_by_date_range() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db();

  Add::new(create_test_paper()).execute(&mut db)?;
  Add::new(create_second_test_paper()).execute(&mut db)?;

  // Remove papers from 2023 only
  let cutoff_date = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
  let removed =
    Remove::from_query(Query::before_date(cutoff_date).order_by(OrderField::PublicationDate))
      .execute(&mut db)?;

  assert_eq!(removed.len(), 1);
  assert_eq!(removed[0].title, "Test Paper");

  Ok(())
}
