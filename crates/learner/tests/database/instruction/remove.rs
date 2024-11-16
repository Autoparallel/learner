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

  #[tokio::test]
  #[traced_test]
  async fn test_remove_existing_paper() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;

    let paper = create_test_paper();
    Add::paper(&paper).execute(&mut db).await?;

    let removed_papers =
      Remove::by_source(paper.source, &paper.source_identifier).execute(&mut db).await?;

    assert_eq!(removed_papers.len(), 1);
    assert_eq!(removed_papers[0].title, paper.title);
    assert_eq!(removed_papers[0].authors.len(), paper.authors.len());

    let results = Query::by_source(paper.source, &paper.source_identifier).execute(&mut db).await?;
    assert_eq!(results.len(), 0);

    Ok(())
  }

  #[tokio::test]
  #[traced_test]
  async fn test_remove_nonexistent_paper() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;

    let removed = Remove::by_source(Source::Arxiv, "nonexistent").execute(&mut db).await?;
    assert!(removed.is_empty());

    Ok(())
  }

  #[tokio::test]
  #[traced_test]
  async fn test_remove_cascades_to_authors() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;

    let paper = create_test_paper();
    Add::paper(&paper).execute(&mut db).await?;

    Remove::from_query(Query::text("test")).execute(&mut db).await?;
    let authors = Query::by_author("").execute(&mut db).await?;

    assert_eq!(authors.len(), 0);
    Ok(())
  }

  #[tokio::test]
  #[traced_test]
  async fn test_remove_complete_paper() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;

    let paper = create_test_paper();
    // Add paper with document
    Add::complete(&paper).execute(&mut db).await?;

    // Remove it
    Remove::by_source(paper.source, &paper.source_identifier).execute(&mut db).await?;

    // Verify paper is gone
    let results = Query::by_source(paper.source, &paper.source_identifier).execute(&mut db).await?;
    assert_eq!(results.len(), 0);

    Ok(())
  }
}

/// Dry run functionality tests
mod dry_run {
  use super::*;

  #[tokio::test]
  #[traced_test]
  async fn test_dry_run_basic() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;

    let paper = create_test_paper();
    Add::paper(&paper).execute(&mut db).await?;

    let would_remove =
      Remove::by_source(paper.source, &paper.source_identifier).dry_run().execute(&mut db).await?;

    assert_eq!(would_remove.len(), 1);
    assert_eq!(would_remove[0].title, paper.title);

    let results = Query::by_source(paper.source, &paper.source_identifier).execute(&mut db).await?;
    assert_eq!(results.len(), 1);

    Ok(())
  }

  #[tokio::test]
  #[traced_test]
  async fn test_dry_run_returns_complete_paper() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;

    let paper = create_test_paper();
    Add::paper(&paper).execute(&mut db).await?;

    let would_remove = Remove::from_query(Query::text("test")).dry_run().execute(&mut db).await?;

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

  #[tokio::test]
  #[traced_test]
  async fn test_dry_run_with_complete_paper() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;

    let paper = create_test_paper();
    Add::complete(&paper).execute(&mut db).await?;

    let would_remove =
      Remove::by_source(paper.source, &paper.source_identifier).dry_run().execute(&mut db).await?;

    // Verify paper would be removed
    assert_eq!(would_remove.len(), 1);

    // But verify it's still in the database
    let results = Query::by_source(paper.source, &paper.source_identifier).execute(&mut db).await?;
    assert_eq!(results.len(), 1);

    Ok(())
  }
}

/// Query-based removal tests
mod query_based_removal {
  use super::*;

  #[tokio::test]
  #[traced_test]
  async fn test_remove_by_text_search() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;

    Add::paper(&create_test_paper()).execute(&mut db).await?;
    Add::paper(&create_second_test_paper()).execute(&mut db).await?;

    let removed = Remove::from_query(Query::text("two")).execute(&mut db).await?;
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].title, "Test Paper: Two");

    Ok(())
  }

  #[tokio::test]
  #[traced_test]
  async fn test_remove_by_author() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;

    Add::paper(&create_test_paper()).execute(&mut db).await?;
    Add::paper(&create_second_test_paper()).execute(&mut db).await?;

    let removed = Remove::from_query(Query::by_author("John Doe")).execute(&mut db).await?;
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].authors[0].name, "John Doe");

    // Verify only the matching paper was removed
    let remaining = Query::list_all().execute(&mut db).await?;
    assert_eq!(remaining.len(), 1);

    Ok(())
  }

  #[tokio::test]
  #[traced_test]
  async fn test_remove_with_ordering() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;

    Add::paper(&create_test_paper()).execute(&mut db).await?;
    Add::paper(&create_second_test_paper()).execute(&mut db).await?;

    let removed =
      Remove::from_query(Query::text("test").order_by(OrderField::PublicationDate).descending())
        .execute(&mut db)
        .await?;

    assert_eq!(removed.len(), 2);
    assert_eq!(removed[0].title, "Test Paper: Two"); // More recent
    assert_eq!(removed[1].title, "Test Paper");

    Ok(())
  }

  #[tokio::test]
  #[traced_test]
  async fn test_remove_by_date_range() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;

    Add::paper(&create_test_paper()).execute(&mut db).await?;
    Add::paper(&create_second_test_paper()).execute(&mut db).await?;

    let cutoff_date = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let removed =
      Remove::from_query(Query::before_date(cutoff_date).order_by(OrderField::PublicationDate))
        .execute(&mut db)
        .await?;

    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].title, "Test Paper");

    Ok(())
  }

  #[tokio::test]
  #[traced_test]
  async fn test_remove_multiple_papers_by_source() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;

    // Add multiple papers from same source
    let paper1 = create_test_paper();
    let paper2 = create_second_test_paper();
    Add::paper(&paper1).execute(&mut db).await?;
    Add::paper(&paper2).execute(&mut db).await?;

    // Use a text search that will match all papers from this source
    // alternatively we could use Query::list_all() with a source filter
    let removed = Remove::from_query(Query::text("test")).execute(&mut db).await?;
    assert_eq!(removed.len(), 2);
    assert!(removed.iter().all(|p| p.source == Source::Arxiv));

    // Verify all papers are gone
    let remaining = Query::text("test").execute(&mut db).await?;
    assert!(remaining.is_empty());

    Ok(())
  }

  // Alternative version using list_all
  #[tokio::test]
  #[traced_test]
  async fn test_remove_multiple_papers_from_source() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;

    // Add papers from different sources
    let paper1 = create_test_paper();
    let paper2 = create_second_test_paper();
    Add::paper(&paper1).execute(&mut db).await?;
    Add::paper(&paper2).execute(&mut db).await?;

    // Verify we have papers from our source before removal
    let initial = Query::list_all().execute(&mut db).await?;
    assert!(initial.iter().any(|p| p.source == Source::Arxiv));

    // Remove all papers using list_all and checking source
    let removed =
      Remove::from_query(Query::list_all().order_by(OrderField::Source)).execute(&mut db).await?;

    // Count papers from our source
    let arxiv_count = removed.iter().filter(|p| p.source == Source::Arxiv).count();
    assert_eq!(arxiv_count, 2);

    // Verify no papers remain from that source
    let remaining = Query::list_all().execute(&mut db).await?;
    assert!(!remaining.iter().any(|p| p.source == Source::Arxiv));

    Ok(())
  }
}

/// Recovery and data integrity tests
mod recovery {
  use super::*;

  #[tokio::test]
  #[traced_test]
  async fn test_remove_papers_can_be_readded() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;

    let paper = create_test_paper();
    Add::paper(&paper).execute(&mut db).await?;

    let removed_papers = Remove::from_query(Query::text("test")).execute(&mut db).await?;
    assert_eq!(removed_papers.len(), 1);

    Add::paper(&removed_papers[0]).execute(&mut db).await?;

    let results = Query::text("test").execute(&mut db).await?;
    assert_eq!(results.len(), 1);

    Ok(())
  }

  #[tokio::test]
  #[traced_test]
  async fn test_bulk_remove_and_readd() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;

    Add::paper(&create_test_paper()).execute(&mut db).await?;
    Add::paper(&create_second_test_paper()).execute(&mut db).await?;

    let removed = Remove::from_query(Query::text("test")).execute(&mut db).await?;
    assert_eq!(removed.len(), 2);

    for paper in &removed {
      Add::paper(paper).execute(&mut db).await?;
    }

    let results = Query::text("test").execute(&mut db).await?;
    assert_eq!(results.len(), 2);

    // Verify order is preserved
    assert_eq!(results[0].title, removed[0].title);
    assert_eq!(results[1].title, removed[1].title);

    Ok(())
  }

  #[tokio::test]
  #[traced_test]
  async fn test_readd_with_different_completion() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;

    // Add paper without document
    let paper = create_test_paper();
    Add::paper(&paper).execute(&mut db).await?;

    // Remove it
    let removed =
      Remove::by_source(paper.source, &paper.source_identifier).execute(&mut db).await?;

    // Readd with document
    Add::complete(&removed[0]).execute(&mut db).await?;

    // Verify paper exists with updated data
    let results = Query::by_source(paper.source, &paper.source_identifier).execute(&mut db).await?;
    assert_eq!(results.len(), 1);

    Ok(())
  }

  #[tokio::test]
  #[traced_test]
  async fn test_remove_and_readd_preserves_metadata() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;

    let mut paper = create_test_paper();
    // Add some optional fields
    paper.doi = Some("10.1234/test".to_string());
    paper.pdf_url = Some("https://example.com/test.pdf".to_string());

    Add::paper(&paper).execute(&mut db).await?;

    let removed =
      Remove::by_source(paper.source, &paper.source_identifier).execute(&mut db).await?;
    Add::paper(&removed[0]).execute(&mut db).await?;

    let results = Query::by_source(paper.source, &paper.source_identifier).execute(&mut db).await?;
    assert_eq!(results[0].doi, paper.doi);
    assert_eq!(results[0].pdf_url, paper.pdf_url);

    Ok(())
  }

  #[tokio::test]
  #[traced_test]
  async fn test_remove_readd_with_updated_data() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;

    let paper = create_test_paper();
    Add::paper(&paper).execute(&mut db).await?;

    let mut removed =
      Remove::by_source(paper.source, &paper.source_identifier).execute(&mut db).await?;

    // Modify the removed paper
    let mut updated_paper = removed.remove(0);
    updated_paper.abstract_text = "Updated abstract".to_string();
    updated_paper.doi = Some("10.1234/new".to_string());

    // Readd with changes
    Add::paper(&updated_paper).execute(&mut db).await?;

    let results = Query::by_source(paper.source, &paper.source_identifier).execute(&mut db).await?;
    assert_eq!(results[0].abstract_text, "Updated abstract");
    assert_eq!(results[0].doi, Some("10.1234/new".to_string()));

    Ok(())
  }
}
