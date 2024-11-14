// use super::*;
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

#[test]
fn test_remove_existing_paper() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db();

  // Add a test paper
  let paper = create_test_paper();
  let source = paper.source.clone();
  let id = paper.source_identifier.clone();
  Add::new(paper.clone()).execute(&mut db)?;

  // Remove it
  let removed_papers = Remove::new(source.clone(), id.clone()).execute(&mut db)?;

  // Verify returned paper matches original
  assert_eq!(removed_papers.len(), 1);
  assert_eq!(removed_papers[0].title, paper.title);
  assert_eq!(removed_papers[0].authors.len(), paper.authors.len());

  // Verify it's gone from database
  let results = Query::by_source(source, id).execute(&mut db)?;
  assert_eq!(results.len(), 0);

  Ok(())
}

#[test]
fn test_remove_nonexistent_paper() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db();

  let removed = Remove::new(Source::Arxiv, "nonexistent").execute(&mut db)?;
  assert!(removed.is_empty());

  Ok(())
}

#[test]
fn test_remove_dry_run() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db();

  // Add a test paper
  let paper = create_test_paper();
  let source = paper.source.clone();
  let id = paper.source_identifier.clone();
  Add::new(paper.clone()).execute(&mut db)?;

  // Try removing with dry run
  let would_remove = Remove::new(source.clone(), id.clone()).dry_run().execute(&mut db)?;

  // Verify returned paper matches original
  assert_eq!(would_remove.len(), 1);
  assert_eq!(would_remove[0].title, paper.title);

  // Verify paper still exists in database
  let results = Query::by_source(source, id).execute(&mut db)?;
  assert_eq!(results.len(), 1);

  Ok(())
}

#[test]
fn test_remove_cascades_to_authors() -> TestResult<()> {
  todo!()
  //   let (mut db, _dir) = setup_test_db();

  //   // Add a test paper
  //   let paper = create_test_paper();
  //   let source = paper.source.clone();
  //   let id = paper.source_identifier.clone();
  //   Add::new(paper).execute(&mut db)?;

  //   // Remove the paper
  //   Remove::new(source, id).execute(&mut db)?;

  //   // Verify no orphaned authors remain
  //   let count: i64 =
  //     db.conn.prepare("SELECT COUNT(*) FROM authors")?.query_row([], |row| row.get(0))?;

  //   assert_eq!(count, 0);
  //   Ok(())
}

#[test]
fn test_remove_papers_can_be_readded() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db();

  // Add original paper
  let paper = create_test_paper();
  let source = paper.source.clone();
  let id = paper.source_identifier.clone();
  Add::new(paper).execute(&mut db)?;

  // Remove and capture the removed papers
  let removed_papers = Remove::new(source.clone(), id.clone()).execute(&mut db)?;
  assert_eq!(removed_papers.len(), 1);

  // Re-add the removed paper
  Add::new(removed_papers[0].clone()).execute(&mut db)?;

  // Verify paper exists again
  let results = Query::by_source(source, id).execute(&mut db)?;
  assert_eq!(results.len(), 1);

  Ok(())
}

#[test]
fn test_dry_run_returns_complete_paper() -> TestResult<()> {
  let (mut db, _dir) = setup_test_db();

  // Add a test paper
  let paper = create_test_paper();
  let source = paper.source.clone();
  let id = paper.source_identifier.clone();
  Add::new(paper.clone()).execute(&mut db)?;

  // Do dry run removal
  let would_remove = Remove::new(source, id).dry_run().execute(&mut db)?;

  // Verify ALL paper data is included
  assert_eq!(would_remove.len(), 1);
  let removed = &would_remove[0];
  assert_eq!(removed.title, paper.title);
  assert_eq!(removed.abstract_text, paper.abstract_text);
  assert_eq!(removed.publication_date, paper.publication_date);
  assert_eq!(removed.source, paper.source);
  assert_eq!(removed.source_identifier, paper.source_identifier);
  assert_eq!(removed.pdf_url, paper.pdf_url);
  assert_eq!(removed.doi, paper.doi);
  assert_eq!(removed.authors.len(), paper.authors.len());

  // Verify author details
  for (removed_author, original_author) in removed.authors.iter().zip(paper.authors.iter()) {
    assert_eq!(removed_author.name, original_author.name);
    assert_eq!(removed_author.affiliation, original_author.affiliation);
    assert_eq!(removed_author.email, original_author.email);
  }

  Ok(())
}
