//! Database instruction implementation for adding papers and documents.
//!
//! This module provides functionality for adding papers and their associated documents
//! to the database. It supports several addition patterns:
//!
//! - Adding paper metadata only
//! - Adding complete papers with documents
//! - Batch addition of documents for existing papers
//!
//! The implementation emphasizes:
//! - Atomic transactions for data consistency
//! - Efficient batch processing
//! - Concurrent document downloads
//! - Duplicate handling
//!
//! # Examples
//!
//! ```no_run
//! use learner::{
//!   database::{Add, Database, Query},
//!   prelude::*,
//!   resource::Paper,
//!   Learner,
//! };
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut db = Database::open(Database::default_path()).await?;
//!
//! // Add just paper metadata
//! # let learner = Learner::builder().build().await?;
//! # let paper = learner.retriever.get_paper("2301.07041").await?;
//! Add::paper(&paper).execute(&mut db).await?;
//!
//! // Add paper with document
//! Add::complete(&paper).execute(&mut db).await?;
//!
//! // Add documents for papers matching a query
//! let query = Query::by_author("Alice Researcher");
//! Add::documents(query).execute(&mut db).await?;
//! # Ok(())
//! # }
//! ```

use std::collections::HashSet;

use futures::future::try_join_all;

use super::*;

// TODO (autoparallel): Would be good to have `Papers` and `Documents` and `Completes` instead,
// possibly, and just have a simple API for single paper calls that just dumps into the 3 variants.
/// Represents different types of additions to the database.
///
/// This enum defines the supported addition operations, each handling a different
/// aspect of paper and document management:
///
/// - Metadata-only additions
/// - Complete paper additions (metadata + document)
/// - Batch document additions for existing papers
#[derive(Debug)]
pub enum Addition<'a> {
  /// Add just the paper metadata without associated documents
  Paper(&'a Paper),
  /// Add both paper metadata and download its associated document
  Complete(&'a Paper),
  /// Add documents for papers matching a specified query
  Documents(Query<'a>),
}

/// Database instruction for adding papers and documents.
///
/// This struct implements the [`DatabaseInstruction`] trait to provide
/// paper and document addition functionality. It handles:
///
/// - Paper metadata insertion
/// - Author information management
/// - Document downloading and storage
/// - Batch processing for multiple papers
///
/// Operations are performed atomically using database transactions to
/// ensure consistency.
pub struct Add<'a> {
  /// The type of addition operation to perform
  addition: Addition<'a>,
}

impl<'a> Add<'a> {
  /// Creates an instruction to add paper metadata only.
  ///
  /// This method creates an addition that will store the paper's metadata
  /// in the database without downloading or storing its associated document.
  ///
  /// # Arguments
  ///
  /// * `paper` - Reference to the paper to add
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::database::Add;
  /// # use learner::paper::Paper;
  /// # use learner::Learner;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// # let learner = Learner::builder().build().await?;
  /// # let retriever = learner.retriever;
  /// let paper = retriever.get_paper("2301.07041").await?;
  /// let instruction = Add::paper(&paper);
  /// # Ok(())
  /// # }
  /// ```
  pub fn paper(paper: &'a Paper) -> Self { Self { addition: Addition::Paper(paper) } }

  /// Creates an instruction to add a complete paper with its document.
  ///
  /// This method creates an addition that will:
  /// 1. Store the paper's metadata
  /// 2. Download the paper's document
  /// 3. Store the document in the configured storage location
  ///
  /// # Arguments
  ///
  /// * `paper` - Reference to the paper to add
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::database::Add;
  /// # use learner::paper::Paper;
  /// # use learner::Learner;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// # let learner = Learner::builder().build().await?;
  /// # let retriever = learner.retriever;
  /// let paper = retriever.get_paper("2301.07041").await?;
  /// let instruction = Add::complete(&paper);
  /// # Ok(())
  /// # }
  /// ```
  pub fn complete(paper: &'a Paper) -> Self { Self { addition: Addition::Complete(paper) } }

  /// Creates an instruction to add documents for papers matching a query.
  ///
  /// This method supports batch document addition by:
  /// 1. Finding papers matching the query
  /// 2. Filtering out papers that already have documents
  /// 3. Concurrently downloading missing documents
  /// 4. Storing documents in the configured location
  ///
  /// # Arguments
  ///
  /// * `query` - Query to identify papers needing documents
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::database::{Add, Query};
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// // Add documents for all papers by an author
  /// let query = Query::by_author("Alice Researcher");
  /// let instruction = Add::documents(query);
  ///
  /// // Or add documents for papers matching a search
  /// let query = Query::text("quantum computing");
  /// let instruction = Add::documents(query);
  /// # Ok(())
  /// # }
  /// ```
  pub fn documents(query: Query<'a>) -> Self { Self { addition: Addition::Documents(query) } }

  /// Converts a paper-only addition to a complete addition.
  ///
  /// This method allows for fluent conversion of a paper metadata addition
  /// to include document download and storage.
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::database::Add;
  /// # use learner::Learner;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// # let learner = Learner::builder().build().await?;
  /// # let retriever = learner.retriever;
  /// let paper = retriever.get_paper("2301.07041").await?;
  /// let instruction = Add::paper(&paper).with_document();
  /// # Ok(())
  /// # }
  /// ```
  pub fn with_document(self) -> Self {
    match self.addition {
      Addition::Paper(paper) => Self { addition: Addition::Complete(paper) },
      _ => self,
    }
  }

  /// Builds the SQL for inserting paper metadata.
  fn build_paper_sql(paper: &Paper) -> (String, Vec<Option<String>>) {
    (
      "INSERT INTO papers (
            title, abstract_text, publication_date,
            source, source_identifier, pdf_url, doi
        ) VALUES (?, ?, ?, ?, ?, ?, ?)"
        .to_string(),
      vec![
        Some(paper.title.clone()),
        Some(paper.abstract_text.clone()),
        Some(paper.publication_date.to_rfc3339()),
        Some(paper.source.to_string()),
        Some(paper.source_identifier.clone()),
        paper.pdf_url.clone(),
        paper.doi.clone(),
      ],
    )
  }

  /// Builds the SQL for inserting author information.
  fn build_author_sql(author: &Author, paper: &Paper) -> (String, Vec<Option<String>>) {
    (
      "INSERT INTO authors (paper_id, name, affiliation, email)
         SELECT id, ?, ?, ?
         FROM papers
         WHERE source = ? AND source_identifier = ?"
        .to_string(),
      vec![
        Some(author.name.clone()),
        author.affiliation.clone(),
        author.email.clone(),
        Some(paper.source.to_string()),
        Some(paper.source_identifier.clone()),
      ],
    )
  }

  /// Builds the SQL for recording document storage information.
  fn build_document_sql(
    paper: &Paper,
    storage_path: &Path,
    filename: &Path,
  ) -> (String, Vec<Option<String>>) {
    (
      "INSERT INTO files (paper_id, path, filename, download_status)
         SELECT p.id, ?, ?, 'Success'
         FROM papers p
         WHERE p.source = ? AND p.source_identifier = ?"
        .to_string(),
      vec![
        Some(storage_path.to_string_lossy().to_string()),
        Some(filename.to_string_lossy().to_string()),
        Some(paper.source.to_string()),
        Some(paper.source_identifier.clone()),
      ],
    )
  }

  /// Builds the SQL for checking existing document records.
  fn build_existing_docs_sql(papers: &[&Paper]) -> (String, Vec<Option<String>>) {
    let mut params = Vec::new();
    let mut param_placeholders = Vec::new();

    for paper in papers {
      params.push(Some(paper.source.to_string()));
      params.push(Some(paper.source_identifier.clone()));
      param_placeholders.push("(? = p.source AND ? = p.source_identifier)");
    }

    (
      format!(
        "SELECT p.source, p.source_identifier
             FROM files f
             JOIN papers p ON p.id = f.paper_id
             WHERE f.download_status = 'Success'
             AND ({})",
        param_placeholders.join(" OR ")
      ),
      params,
    )
  }
}

#[async_trait]
impl DatabaseInstruction for Add<'_> {
  type Output = Vec<Paper>;

  async fn execute(&self, db: &mut Database) -> Result<Self::Output> {
    match &self.addition {
      Addition::Paper(paper) => {
        // Check for existing paper
        if Query::by_source(&paper.source, &paper.source_identifier)
          .execute(db)
          .await?
          .into_iter()
          .next()
          .is_some()
        {
          return Err(LearnerError::DatabaseDuplicatePaper(paper.title.clone()));
        }

        let (paper_sql, paper_params) = Self::build_paper_sql(paper);
        let author_statements: Vec<_> =
          paper.authors.iter().map(|author| Self::build_author_sql(author, paper)).collect();

        db.conn
          .call(move |conn| {
            let tx = conn.transaction()?;
            tx.execute(&paper_sql, params_from_iter(paper_params))?;

            for (author_sql, author_params) in author_statements {
              tx.execute(&author_sql, params_from_iter(author_params))?;
            }

            tx.commit()?;
            Ok(())
          })
          .await?;

        Ok(vec![(*paper).clone()])
      },

      Addition::Complete(paper) => {
        // Add paper first
        if let Err(LearnerError::DatabaseDuplicatePaper(_)) = Add::paper(paper).execute(db).await {
          warn!(
            "Tried to add complete paper when paper existed in database already, attempting to \
             add only the document!"
          )
        };

        // Add document
        let storage_path = db.get_storage_path().await?;
        let filename = paper.download_pdf(&storage_path).await?;

        let (doc_sql, doc_params) = Self::build_document_sql(paper, &storage_path, &filename);

        db.conn
          .call(move |conn| {
            let tx = conn.transaction()?;
            tx.execute(&doc_sql, params_from_iter(doc_params))?;
            tx.commit()?;
            Ok(())
          })
          .await?;

        Ok(vec![(*paper).clone()])
      },

      Addition::Documents(query) => {
        let papers = query.execute(db).await?;
        if papers.is_empty() {
          return Ok(Vec::new());
        }

        let storage_path = db.get_storage_path().await?;
        let mut added = Vec::new();

        // Process papers in batches
        for chunk in papers.chunks(10) {
          // Check which papers already have documents
          let paper_refs: Vec<_> = chunk.iter().collect();
          let (check_sql, check_params) = Self::build_existing_docs_sql(&paper_refs);

          let existing_docs: HashSet<(String, String)> = db
            .conn
            .call(move |conn| {
              let mut docs = HashSet::new();
              let mut stmt = conn.prepare_cached(&check_sql)?;
              let mut rows = stmt.query(params_from_iter(check_params))?;

              while let Some(row) = rows.next()? {
                docs.insert((row.get::<_, String>(0)?, row.get::<_, String>(1)?));
              }
              Ok(docs)
            })
            .await?;

          // Create future for each paper that needs downloading
          let download_futures: Vec<_> = chunk
            .iter()
            .filter(|paper| {
              let key = (paper.source.to_string(), paper.source_identifier.clone());
              !existing_docs.contains(&key)
            })
            .map(|paper| {
              let paper = paper.clone();
              let storage_path = storage_path.clone();
              async move { paper.download_pdf(&storage_path).await.map(|f| (paper, f)) }
            })
            .collect();

          if download_futures.is_empty() {
            continue;
          }

          // Download PDFs concurrently and collect results
          let results = try_join_all(download_futures).await?;

          // Prepare batch insert for successful downloads
          let mut insert_sqls = Vec::new();
          let mut insert_params = Vec::new();

          for (paper, filename) in results {
            let (sql, params) = Self::build_document_sql(&paper, &storage_path, &filename);
            insert_sqls.push(sql);
            insert_params.extend(params);
            added.push(paper);
          }

          if !insert_sqls.is_empty() {
            // Execute batch insert
            db.conn
              .call(move |conn| {
                let tx = conn.transaction()?;
                for (sql, params) in insert_sqls.iter().zip(insert_params.chunks(4)) {
                  tx.execute(sql, params_from_iter(params))?;
                }
                tx.commit()?;
                Ok(())
              })
              .await?;
          }
        }

        Ok(added)
      },
    }
  }
}
