use super::*;

/// Basic paper addition tests
mod basic_operations {

  use super::*;

  #[traced_test]
  #[tokio::test]
  async fn test_add_paper() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;
    let paper = create_test_paper();

    let papers = Add::paper(&paper).execute(&mut learner.database).await?;
    assert_eq!(papers.len(), 1);
    assert_eq!(papers[0].title, paper.title);

    // Verify paper exists in database
    let stored = Query::by_source(&paper.source, &paper.source_identifier)
      .execute(&mut learner.database)
      .await?;
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].title, paper.title);

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_add_paper_twice() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;
    let paper = create_test_paper();

    Add::paper(&paper).execute(&mut learner.database).await?;
    let err = Add::paper(&paper).execute(&mut learner.database).await.unwrap_err();

    assert!(matches!(err, LearnerError::DatabaseDuplicatePaper(_)));

    // Verify only one copy exists
    let stored = Query::list_all().execute(&mut learner.database).await?;
    assert_eq!(stored.len(), 1);

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_add_paper_with_authors() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;
    let mut paper = create_test_paper();
    paper.authors = vec![
      Author {
        name:        "Test Author 1".into(),
        affiliation: Some("University 1".into()),
        email:       Some("email1@test.com".into()),
      },
      Author { name: "Test Author 2".into(), affiliation: None, email: None },
    ];

    Add::paper(&paper).execute(&mut learner.database).await?;

    // Verify authors were stored
    let stored = Query::by_author("Test Author 1").execute(&mut learner.database).await?;
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
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;
    let paper = learner.retriever.get_paper("https://arxiv.org/abs/2301.07041").await?;

    let papers = Add::complete(&paper).execute(&mut learner.database).await?;
    assert_eq!(papers.len(), 1);

    // Verify both paper and document were added
    let stored = Query::by_source(&paper.source, &paper.source_identifier)
      .execute(&mut learner.database)
      .await?;
    assert_eq!(stored.len(), 1);

    // Verify PDF exists in storage location
    let storage_path = learner.database.get_storage_path().await?;
    let pdf_path = storage_path.join(paper.filename());
    assert!(pdf_path.exists(), "PDF file should exist at {:?}", pdf_path);

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_add_paper_then_document() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;
    let paper = learner.retriever.get_paper("https://arxiv.org/abs/2301.07041").await?;

    // First add paper only
    Add::paper(&paper).execute(&mut learner.database).await?;

    // Then add with document
    let papers = Add::complete(&paper).execute(&mut learner.database).await?;
    assert_eq!(papers.len(), 1);

    // Verify PDF exists
    let storage_path = learner.database.get_storage_path().await?;
    let pdf_path = storage_path.join(paper.filename());
    assert!(pdf_path.exists());

    assert!(logs_contain(
      "WARN test_add_paper_then_document: learner::database::instruction::add: Tried to add \
       complete paper when paper existed in database already, attempting to add only the document!"
    ));
    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_chain_document_addition() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;
    let paper = learner.retriever.get_paper("https://arxiv.org/abs/2301.07041").await?;

    let papers = Add::paper(&paper).with_document().execute(&mut learner.database).await?;
    assert_eq!(papers.len(), 1);

    // Verify PDF exists
    let storage_path = learner.database.get_storage_path().await?;
    let pdf_path = storage_path.join(paper.filename());
    assert!(pdf_path.exists());

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_add_documents_by_query() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;

    // Add multiple papers without documents
    let paper1 = learner.retriever.get_paper("https://arxiv.org/abs/2301.07041").await?;
    let paper2 = learner.retriever.get_paper("https://eprint.iacr.org/2016/260").await?;
    Add::paper(&paper1).execute(&mut learner.database).await?;
    Add::paper(&paper2).execute(&mut learner.database).await?;

    // Add documents for all papers
    let papers = Add::documents(Query::list_all()).execute(&mut learner.database).await?;
    assert_eq!(papers.len(), 2);

    // Verify PDFs exist
    let storage_path = learner.database.get_storage_path().await?;
    for paper in papers {
      let pdf_path = storage_path.join(paper.filename());
      assert!(pdf_path.exists(), "PDF should exist for {}", paper.source_identifier);
    }

    Ok(())
  }
}

/// Edge case tests
mod edge_cases {
  use super::*;

  #[traced_test]
  #[tokio::test]
  async fn test_add_paper_with_special_characters() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;
    let mut paper = create_test_paper();
    paper.title = "Test & Paper: A Study!".into();
    paper.abstract_text = "Abstract with & and other symbols: @#$%".into();

    let papers = Add::paper(&paper).execute(&mut learner.database).await?;
    assert_eq!(papers.len(), 1);
    assert_eq!(papers[0].title, paper.title);

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_add_empty_author_list() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;
    let mut paper = create_test_paper();
    paper.authors.clear();

    let papers = Add::paper(&paper).execute(&mut learner.database).await?;
    assert_eq!(papers.len(), 1);
    assert!(papers[0].authors.is_empty());

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_add_paper_with_optional_fields() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;
    let mut paper = create_test_paper();
    paper.doi = Some("10.1234/test".into());
    paper.pdf_url = Some("https://example.com/paper.pdf".into());

    let papers = Add::paper(&paper).execute(&mut learner.database).await?;
    assert_eq!(papers[0].doi, Some("10.1234/test".into()));
    assert_eq!(papers[0].pdf_url, Some("https://example.com/paper.pdf".into()));

    Ok(())
  }
}
