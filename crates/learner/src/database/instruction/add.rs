use super::*;

/// Database instruction for adding records
pub struct Add<'a> {
  /// The type of addition operation to perform
  record: &'a Record,
}

impl<'a> Add<'a> {
  /// Creates an instruction to add a complete record
  pub fn record(record: &'a Record) -> Self { Self { record } }

  /// Builds the SQL for inserting a record
  fn build_record_sql(record: &Record) -> (String, Vec<impl ToSql>) {
    (
      "INSERT INTO resources (
                identifier, resource_type,
                resource, state, storage, retrieval
            ) VALUES (?, ?, ?, ?, ?, ?)"
        .to_string(),
      vec![
        // We'll use the title as the identifier for now
        record.resource.title.clone(),
        // For now all records are "paper" type
        "paper".to_string(),
        // Serialize each component to JSON
        serde_json::to_string(&record.resource).unwrap(),
        serde_json::to_string(&record.state).unwrap(),
        serde_json::to_string(&record.storage).unwrap(),
        serde_json::to_string(&record.retrieval).unwrap(),
      ],
    )
  }
}

#[async_trait]
impl DatabaseInstruction for Add<'_> {
  type Output = Record;

  async fn execute(&self, db: &mut Database) -> Result<Self::Output> {
    // Check for existing record
    let (sql, params) = Self::build_record_sql(self.record);

    db.conn
      .call(move |conn| {
        let tx = conn.transaction()?;
        tx.execute(&sql, params_from_iter(params))?;
        tx.commit()?;
        Ok(())
      })
      .await?;

    Ok((*self.record).clone())
  }
}
