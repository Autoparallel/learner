use chrono::Datelike;

use super::*;

/// Basic paper search functionality
mod paper_search {
  use super::*;

  #[tokio::test]
  #[traced_test]
  async fn test_basic_paper_search() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;
    let paper = create_test_paper();
    Add::paper(&paper).execute(&mut learner.database).await?;

    let results = Query::by_paper(&paper).execute(&mut learner.database).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Test Paper");
    Ok(())
  }
}

/// Basic text search functionality
mod text_search {
  use super::*;

  #[tokio::test]
  #[traced_test]
  async fn test_basic_text_search() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;
    let paper = create_test_paper();
    Add::paper(&paper).execute(&mut learner.database).await?;

    let results = Query::text("test paper").execute(&mut learner.database).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Test Paper");
    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_case_insensitive_search() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;
    let paper = create_test_paper();
    Add::paper(&paper).execute(&mut learner.database).await?;

    let results = Query::text("TEST PAPER").execute(&mut learner.database).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Test Paper");

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_word_boundaries() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;

    let mut paper = create_test_paper();
    paper.title = "Testing Paper".to_string();
    Add::paper(&paper).execute(&mut learner.database).await?;

    let results = Query::text("test").execute(&mut learner.database).await?;
    assert_eq!(results.len(), 1);

    let results = Query::text("testing").execute(&mut learner.database).await?;
    assert_eq!(results.len(), 1);

    let results = Query::text("est").execute(&mut learner.database).await?;
    assert_eq!(results.len(), 0, "Partial word match should not work");

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_abstract_search() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;
    let mut paper = create_test_paper();
    paper.abstract_text = "This is a unique phrase in the abstract".to_string();
    Add::paper(&paper).execute(&mut learner.database).await?;

    // Search should only match title by default since that's what we indexed
    let results = Query::text("unique phrase").execute(&mut learner.database).await?;
    assert_eq!(results.len(), 0);

    // Search for title instead
    let results = Query::text("test paper").execute(&mut learner.database).await?;
    assert_eq!(results.len(), 1);

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_multiple_term_search() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;
    let mut paper = create_test_paper();
    paper.title = "Machine Learning Research".to_string();
    paper.abstract_text = "A study about neural networks".to_string();
    Add::paper(&paper).execute(&mut learner.database).await?;

    // Each term should be searched independently in title
    let results = Query::text("machine").execute(&mut learner.database).await?;
    assert_eq!(results.len(), 1, "Should match single term in title");

    let results = Query::text("learning research").execute(&mut learner.database).await?;
    assert_eq!(results.len(), 1, "Should match multiple terms in title");

    // Abstract text isn't searched
    let results = Query::text("neural").execute(&mut learner.database).await?;
    assert_eq!(results.len(), 0, "Should not match terms in abstract");

    Ok(())
  }
}

/// Author search functionality
mod author_search {
  use super::*;

  #[tokio::test]
  async fn test_exact_author_match() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;
    let paper = create_test_paper();
    Add::paper(&paper).execute(&mut learner.database).await?;

    let results = Query::by_author("John Doe").execute(&mut learner.database).await?;
    assert_eq!(results.len(), 1);
    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_partial_author_name() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;
    let mut paper = create_test_paper();
    paper.authors =
      vec![Author { name: "John Smith".to_string(), affiliation: None, email: None }, Author {
        name:        "Jane Smith".to_string(),
        affiliation: None,
        email:       None,
      }];
    Add::paper(&paper).execute(&mut learner.database).await?;

    let results = Query::by_author("Smith").execute(&mut learner.database).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].authors.len(), 2);

    let results = Query::by_author("SMITH").execute(&mut learner.database).await?;
    assert_eq!(results.len(), 1, "Author search should be case insensitive");

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_multiple_papers_same_author() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;

    let mut paper1 = create_test_paper();
    let mut paper2 = create_second_test_paper();

    // Give both papers the same author
    let author =
      Author { name: "Shared Author".to_string(), affiliation: None, email: None };
    paper1.authors = vec![author.clone()];
    paper2.authors = vec![author];

    Add::paper(&paper1).execute(&mut learner.database).await?;
    Add::paper(&paper2).execute(&mut learner.database).await?;

    let results = Query::by_author("Shared Author").execute(&mut learner.database).await?;
    assert_eq!(results.len(), 2);

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_author_with_affiliation() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;

    let mut paper = create_test_paper();
    paper.authors = vec![Author {
      name:        "John Doe".to_string(),
      affiliation: Some("Test University".to_string()),
      email:       Some("john@test.edu".to_string()),
    }];

    Add::paper(&paper).execute(&mut learner.database).await?;

    let results = Query::by_author("John Doe").execute(&mut learner.database).await?;
    assert_eq!(results[0].authors[0].affiliation, Some("Test University".to_string()));
    assert_eq!(results[0].authors[0].email, Some("john@test.edu".to_string()));

    Ok(())
  }
}

/// Source-based search functionality
mod source_search {
  use super::*;

  #[traced_test]
  #[tokio::test]
  async fn test_basic_source_search() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;
    let paper = create_test_paper();
    Add::paper(&paper).execute(&mut learner.database).await?;

    let results = Query::by_source("arxiv", "2301.00000").execute(&mut learner.database).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].source_identifier, "2301.00000");
    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_multiple_sources() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;

    let mut paper1 = create_test_paper();
    let mut paper2 = create_second_test_paper();
    paper1.source = "arxiv".to_string();
    paper2.source = "doi".to_string();

    Add::paper(&paper1).execute(&mut learner.database).await?;
    Add::paper(&paper2).execute(&mut learner.database).await?;

    let results =
      Query::list_all().order_by(OrderField::Source).execute(&mut learner.database).await?;
    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|p| p.source == "arxiv"));
    assert!(results.iter().any(|p| p.source == "doi"));

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_source_with_doi() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;

    let mut paper = create_test_paper();
    paper.source = "doi".to_string();
    paper.source_identifier = "10.1234/test".to_string();
    paper.doi = Some(paper.source_identifier.clone());

    Add::paper(&paper).execute(&mut learner.database).await?;

    let results = Query::by_source("doi", "10.1234/test").execute(&mut learner.database).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].doi, Some("10.1234/test".to_string()));

    Ok(())
  }
}

/// Ordering and pagination tests
mod ordering {
  use super::*;

  #[traced_test]
  #[tokio::test]
  async fn test_date_ordering() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;

    let paper1 = create_test_paper(); // 2023
    let paper2 = create_second_test_paper(); // 2024

    Add::paper(&paper1).execute(&mut learner.database).await?;
    Add::paper(&paper2).execute(&mut learner.database).await?;

    let results = Query::list_all()
      .order_by(OrderField::PublicationDate)
      .execute(&mut learner.database)
      .await?;
    assert_eq!(results[0].publication_date.year(), 2023);
    assert_eq!(results[1].publication_date.year(), 2024);

    let results = Query::list_all()
      .order_by(OrderField::PublicationDate)
      .descending()
      .execute(&mut learner.database)
      .await?;
    assert_eq!(results[0].publication_date.year(), 2024);
    assert_eq!(results[1].publication_date.year(), 2023);

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_title_ordering() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;

    let mut paper1 = create_test_paper();
    let mut paper2 = create_second_test_paper();
    paper1.title = "Beta Paper".to_string();
    paper2.title = "Alpha Paper".to_string();

    Add::paper(&paper1).execute(&mut learner.database).await?;
    Add::paper(&paper2).execute(&mut learner.database).await?;

    let results =
      Query::list_all().order_by(OrderField::Title).execute(&mut learner.database).await?;
    assert_eq!(results[0].title, "Alpha Paper");
    assert_eq!(results[1].title, "Beta Paper");

    Ok(())
  }
}

/// Edge cases and special conditions
mod edge_cases {
  use super::*;

  #[traced_test]
  #[tokio::test]
  async fn test_empty_database() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;

    let results = Query::list_all().execute(&mut learner.database).await?;
    assert_eq!(results.len(), 0);

    let results = Query::text("any text").execute(&mut learner.database).await?;
    assert_eq!(results.len(), 0);

    let results = Query::by_author("any author").execute(&mut learner.database).await?;
    assert_eq!(results.len(), 0);

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_special_characters() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;

    let mut paper = create_test_paper();
    // Use simpler special characters that FTS5 can handle
    paper.title = "Test Paper: A Study".to_string();
    paper.authors = vec![Author {
      name:        "O'Connor Smith".to_string(),
      affiliation: None,
      email:       None,
    }];

    Add::paper(&paper).execute(&mut learner.database).await?;

    // Search with and without special characters
    let results = Query::text("Test Paper").execute(&mut learner.database).await?;
    assert_eq!(results.len(), 1);

    let results = Query::text("Test").execute(&mut learner.database).await?;
    assert_eq!(results.len(), 1);

    // Author search should still work with apostrophe
    let results = Query::by_author("O'Connor").execute(&mut learner.database).await?;
    assert_eq!(results.len(), 1);

    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_very_long_text() -> TestResult<()> {
    let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;

    let mut paper = create_test_paper();
    paper.title = "A ".repeat(500) + "unique marker";

    Add::paper(&paper).execute(&mut learner.database).await?;

    let results = Query::text("unique marker").execute(&mut learner.database).await?;
    assert_eq!(results.len(), 1);

    Ok(())
  }
}

#[traced_test]
#[tokio::test]
async fn test_fts_behavior() -> TestResult<()> {
  let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;

  let mut paper = create_test_paper();
  paper.title = "Testing: Advanced Search & Queries".to_string();
  paper.abstract_text = "This is a complex abstract with many terms".to_string();
  Add::paper(&paper).execute(&mut learner.database).await?;

  // Basic word search works
  let results = Query::text("Testing").execute(&mut learner.database).await?;
  assert_eq!(results.len(), 1);

  // Words are tokenized properly
  let results = Query::text("Advanced Search").execute(&mut learner.database).await?;
  assert_eq!(results.len(), 1);

  // Special characters are treated as word boundaries
  let results = Query::text("Queries").execute(&mut learner.database).await?;
  assert_eq!(results.len(), 1);

  // Only title is searchable
  let results = Query::text("complex abstract").execute(&mut learner.database).await?;
  assert_eq!(results.len(), 0);

  Ok(())
}
