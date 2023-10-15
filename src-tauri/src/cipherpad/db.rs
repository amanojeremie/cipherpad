use std::sync::Arc;

use anyhow::{Context, bail};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{ToSql, params_from_iter, types::{Value, FromSql, FromSqlError}, blob::Blob};
pub struct DatabasePool {
  pool: Arc<Pool<SqliteConnectionManager>>
}

type SqlParam = Box<dyn ToSql + Sync + Send>;
type SqlParams = Vec<SqlParam>;

impl DatabasePool {
  pub fn new(db_path: &str) -> Result<Self, anyhow::Error> {
    let manager = SqliteConnectionManager::file(db_path);
    let pool = Pool::new(manager)?;
    Ok(Self { pool: Arc::new(pool) })
  }

  pub async fn execute_query(&self, query: &str, params: SqlParams) -> Result<usize, anyhow::Error> {
    let pool = self.pool.clone();
    let query = query.to_string();
    let params = params_from_iter(params);
    
    tokio::task::spawn_blocking(move || {
      let conn = pool.get().context("Error getting DB connection")?;
      conn.execute(&query, params).context("Error executing statement")
    }).await?
  }

  pub async fn select_query(&self, query: &str, params: SqlParams, columns: usize) -> Result<Vec<Vec<Value>>, anyhow::Error> {
    let pool = self.pool.clone();
    let query = query.to_string();
    let params = params_from_iter(params);
    
    tokio::task::spawn_blocking(move || {
      let conn = pool.get().context("Error getting DB connection")?;
      let mut statement = conn.prepare(&query).context("Error preparing query")?;
      let mut rows = statement.query(params).context("Error executing query")?;
      let mut rows_return = Vec::new();
      while let Some(row) = rows.next().context("Error reading row")? {
        let mut row_vec = Vec::new();
        for i in 0..columns {
          let value = row.get(i).context("Error reading column")?;
          row_vec.push(value);
        }
        rows_return.push(row_vec);
      }
      Ok(rows_return)
    }).await?
  
  }

  pub async fn select_query_single(&self, query: &str, params: SqlParams, columns: usize) -> Result<Vec<Value>, anyhow::Error> {
    let select_result = self.select_query(query, params, columns).await?;
    match select_result.len() {
      1 => Ok(select_result[0].clone()),
      _ => bail!("Error selecting single, received {}", select_result.len())
    }
  }

  pub async fn begin_transaction(&self) -> Result<(), anyhow::Error> {
    self.execute_query("BEGIN TRANSACTION;", SqlParamsBuilder::new().build()).await?;
    Ok(())
  }

  pub async fn commit(&self) -> Result<(), anyhow::Error> {
    self.execute_query("COMMIT;", SqlParamsBuilder::new().build()).await?;
    Ok(())
  }


  pub async fn open_blob<F, R>(&self, row_id: i64, column: &str, table: &str, read_only: bool, func: F) -> Result<R, anyhow::Error> 
  where
    F: FnOnce(Blob) -> Result<R, anyhow::Error> + Send + 'static,
    R: Send + 'static 
  {
    let pool = self.pool.clone();
    let column = column.to_string();
    let table = table.to_string();


    tokio::task::spawn_blocking(move || {
      let conn = pool.get().context("Error getting DB connection")?;
      let blob = conn.blob_open(rusqlite::DatabaseName::Main, &table, &column, row_id, read_only).context("Error opening blob")?;
      Ok(func(blob)?)
    }).await?
  }
}


pub struct SqlParamsBuilder {
  params: SqlParams
}

impl SqlParamsBuilder {
  pub fn new() -> Self {
    Self { params: Vec::new() }
  }

  pub fn add_param<P: ToSql + Sync + Send + 'static>(&mut self, param: P) -> &mut Self {
    self.params.push(Box::new(param));
    self
  }

  pub fn build(&mut self) -> SqlParams {
    std::mem::take(&mut self.params)
  }
}

pub fn value_from_sql<P: FromSql>(value: Option<&Value>) -> Result<P, FromSqlError> {
  Ok(P::column_result(value.into())?)
}