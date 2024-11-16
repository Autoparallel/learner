use add::Add;
use chrono::{TimeZone, Utc};
use learner::{database::*, error::LearnerError, paper::Author};
use query::Query;

use super::setup_test_db;
use crate::{create_second_test_paper, create_test_paper, traced_test, TestResult};

/// Basic paper addition tests
mod basic_operations {

  use super::*;

  #[traced_test]
  #[tokio::test]
  async fn test_add_paper() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;
    let paper = create_test_paper();

    let papers = Add::paper(&paper).execute(&mut db).await?;
    assert_eq!(papers.len(), 1);
    assert_eq!(papers[0].title, paper.title);

    // Verify paper exists in database
    let stored = Query::by_source(paper.source, &paper.source_identifier).execute(&mut db).await?;
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].title, paper.title);

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_add_paper_twice() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;
    let paper = create_test_paper();

    Add::paper(&paper).execute(&mut db).await?;
    let err = Add::paper(&paper).execute(&mut db).await.unwrap_err();

    assert!(matches!(err, LearnerError::DatabaseDuplicatePaper(_)));

    // Verify only one copy exists
    let stored = Query::list_all().execute(&mut db).await?;
    assert_eq!(stored.len(), 1);

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_add_paper_with_authors() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;
    let mut paper = create_test_paper();
    paper.authors = vec![
      Author {
        name:        "Test Author 1".into(),
        affiliation: Some("University 1".into()),
        email:       Some("email1@test.com".into()),
      },
      Author { name: "Test Author 2".into(), affiliation: None, email: None },
    ];

    Add::paper(&paper).execute(&mut db).await?;

    // Verify authors were stored
    let stored = Query::by_author("Test Author 1").execute(&mut db).await?;
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].authors.len(), 2);
    assert_eq!(stored[0].authors[0].affiliation, Some("University 1".into()));
    assert_eq!(stored[0].authors[1].name, "Test Author 2");

    Ok(())
  }
}

/// Tests for paper addition with documents
mod document_operations {
  use learner::paper::Paper;

  use super::*;

  #[traced_test]
  #[tokio::test]
  async fn test_add_complete_paper() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;
    let paper = Paper::new("https://arxiv.org/abs/2301.07041").await.unwrap();

    let papers = Add::complete(&paper).execute(&mut db).await?;
    assert_eq!(papers.len(), 1);

    // Verify both paper and document were added
    let stored = Query::by_source(paper.source, &paper.source_identifier).execute(&mut db).await?;
    assert_eq!(stored.len(), 1);

    // Verify PDF exists in storage location
    let storage_path = db.get_storage_path().await?;
    let pdf_path = storage_path.join(paper.filename());
    assert!(pdf_path.exists(), "PDF file should exist at {:?}", pdf_path);

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_add_paper_then_document() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;
    let paper = Paper::new("https://arxiv.org/abs/2301.07041").await.unwrap();

    // First add paper only
    Add::paper(&paper).execute(&mut db).await?;

    // Then add with document
    let papers = Add::complete(&paper).execute(&mut db).await?;
    assert_eq!(papers.len(), 1);

    // Verify PDF exists
    let storage_path = db.get_storage_path().await?;
    let pdf_path = storage_path.join(paper.filename());
    assert!(pdf_path.exists());

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_chain_document_addition() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;
    let paper = Paper::new("https://arxiv.org/abs/2301.07041").await.unwrap();

    let papers = Add::paper(&paper).with_document().execute(&mut db).await?;
    assert_eq!(papers.len(), 1);

    // Verify PDF exists
    let storage_path = db.get_storage_path().await?;
    let pdf_path = storage_path.join(paper.filename());
    assert!(pdf_path.exists());

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_add_documents_by_query() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;

    // Add multiple papers without documents
    let paper1 = Paper::new("https://arxiv.org/abs/2301.07041").await.unwrap();
    let paper2 = Paper::new("https://eprint.iacr.org/2016/260").await?;
    Add::paper(&paper1).execute(&mut db).await?;
    Add::paper(&paper2).execute(&mut db).await?;

    // Add documents for all papers
    let papers = Add::documents(Query::list_all()).execute(&mut db).await?;
    assert_eq!(papers.len(), 2);

    // Verify PDFs exist
    let storage_path = db.get_storage_path().await?;
    for paper in papers {
      let pdf_path = storage_path.join(paper.filename());
      assert!(pdf_path.exists(), "PDF should exist for {}", paper.source_identifier);
    }

    Ok(())
  }
}

/// Recovery and error handling tests
mod error_handling {
  use super::*;

  #[traced_test]
  #[tokio::test]
  async fn test_add_paper_without_storage_path() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;
    let paper = create_test_paper();

    // Try to add complete paper without setting storage path
    // This should still succeed for paper metadata but fail for PDF
    let result = Add::complete(&paper).execute(&mut db).await;
    assert!(result.is_err());

    // Verify paper wasn't added
    let stored = Query::by_source(paper.source, &paper.source_identifier).execute(&mut db).await?;
    assert_eq!(stored.len(), 0);

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_add_paper_with_invalid_storage_path() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;
    let paper = create_test_paper();

    // Set invalid storage path
    db.set_storage_path("/nonexistent/path").await?;

    // Should fail when trying to save PDF
    let result = Add::complete(&paper).execute(&mut db).await;
    assert!(result.is_err());

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_partial_document_addition() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;

    // Add two papers
    let paper1 = create_test_paper();
    let paper2 = create_second_test_paper();
    Add::paper(&paper1).execute(&mut db).await?;
    Add::paper(&paper2).execute(&mut db).await?;

    // Set invalid path after first paper
    assert!(db.set_storage_path("/nonexistent/path").await.is_err());

    // Should fail but not roll back previous successes
    let result = Add::documents(Query::list_all()).execute(&mut db).await;
    assert!(result.is_ok());

    Ok(())
  }
}

/// Edge case tests
mod edge_cases {
  use super::*;

  #[traced_test]
  #[tokio::test]
  async fn test_add_paper_with_special_characters() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;
    let mut paper = create_test_paper();
    paper.title = "Test & Paper: A Study!".into();
    paper.abstract_text = "Abstract with & and other symbols: @#$%".into();

    let papers = Add::paper(&paper).execute(&mut db).await?;
    assert_eq!(papers.len(), 1);
    assert_eq!(papers[0].title, paper.title);

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_add_empty_author_list() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;
    let mut paper = create_test_paper();
    paper.authors.clear();

    let papers = Add::paper(&paper).execute(&mut db).await?;
    assert_eq!(papers.len(), 1);
    assert!(papers[0].authors.is_empty());

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_add_paper_with_optional_fields() -> TestResult<()> {
    let (mut db, _dir) = setup_test_db().await;
    let mut paper = create_test_paper();
    paper.doi = Some("10.1234/test".into());
    paper.pdf_url = Some("https://example.com/paper.pdf".into());

    let papers = Add::paper(&paper).execute(&mut db).await?;
    assert_eq!(papers[0].doi, Some("10.1234/test".into()));
    assert_eq!(papers[0].pdf_url, Some("https://example.com/paper.pdf".into()));

    Ok(())
  }
}
