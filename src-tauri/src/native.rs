/// Importing the `DbManagerTrait` trait from the `db_manager` module.
use crate::db_manager::DbManagerTrait;
/// Importing the `ColumnInfo` struct.
use crate::ColumnInfo;
/// Importing the `SerializableValue` enum.
use crate::SerializableValue;
/// Importing the `TableRequest` struct.
use crate::TableRequest;
/// Importing the `Connection` and `Result` types from the `rusqlite` crate.
use rusqlite::{Connection, Result};

/// The `NativeDbManager` struct, which represents a connection to a SQLite database.
pub struct NativeDbManager {
    /// The SQLite connection.
    conn: Connection,
}

/// Implementation of `NativeDbManager`.
impl NativeDbManager {
    /// Creates a new `NativeDbManager`.
    ///
    /// # Arguments
    ///
    /// * `conn` - A `Connection` representing the SQLite connection.
    ///
    /// # Returns
    ///
    /// * `NativeDbManager` - The new `NativeDbManager`.
    pub fn new(conn: Connection) -> Self {
        NativeDbManager { conn }
    }
}

/// Implementation of `DbManagerTrait` for `NativeDbManager`.
impl DbManagerTrait for NativeDbManager {
    /// Fetches the table data for a given table name.
    ///
    /// # Arguments
    ///
    /// * `table_name` - A string slice that holds the name of the table.
    ///
    /// # Returns
    ///
    /// * `Result<TableRequest, String>` - The result of the table request.
    fn get_table_data(&mut self, table_name: &str) -> Result<TableRequest, String> {
        println!("Getting Native table data for: {:?}", table_name);
        let mut stmt = match self
            .conn
            .prepare(&format!("SELECT * FROM '{}' LIMIT 100", table_name))
        {
            Ok(stmt) => stmt,
            Err(e) => return Err(e.to_string()),
        };
        println!("Got Native table data for: {:?}", table_name);
        let total_cols = stmt.column_count();
        let rows: Result<Vec<Vec<SerializableValue>>, _> = stmt
            .query_map([], |row| {
                let mut cols = Vec::new();
                for i in 0..total_cols {
                    let value: rusqlite::types::Value = row.get(i)?;
                    cols.push(SerializableValue::from(value));
                }
                Ok(cols)
            })
            .unwrap()
            .collect();

        match rows.as_ref() {
            Ok(rows) => match rows.first() {
                Some(first_item) => {
                    let column_names: Vec<ColumnInfo> = stmt
                        .column_names()
                        .iter()
                        .zip(first_item)
                        .map(|(str, value)| ColumnInfo {
                            name: str.to_string(),
                            type_name: match value {
                                SerializableValue::Null => "NULL".to_string(),
                                SerializableValue::Integer(_) => "INTEGER".to_string(),
                                SerializableValue::Real(_) => "REAL".to_string(),
                                SerializableValue::Text(_) => "TEXT".to_string(),
                                SerializableValue::Blob(_) => "BLOB".to_string(),
                            },
                        })
                        .collect();

                    let total_rows_in_table_from_query = match self.conn.query_row(
                        &format!("SELECT COUNT(*) FROM '{}'", table_name),
                        [],
                        |row| row.get(0),
                    ) {
                        Ok(count) => count,
                        Err(e) => return Err(e.to_string()),
                    };

                    println!(
                        "Total rows in table from query: {:?}",
                        total_rows_in_table_from_query
                    );

                    Ok(TableRequest {
                        column_names,
                        rows: rows.clone(),
                        row_count: total_rows_in_table_from_query,
                    })
                }
                None => Ok(TableRequest {
                    column_names: vec![],
                    rows: vec![],
                    row_count: 0,
                }),
            },
            Err(e) => Err(e.to_string()),
        }
    }

    /// Fetches all table names from the SQLite database.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<String>, String>` - The result of the table names request.
    fn get_all_tables(&mut self) -> Result<Vec<String>, String> {
        let mut stmt = match self
            .conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
        {
            Ok(stmt) => stmt,
            Err(e) => return Err(e.to_string()),
        };
        let rows: Result<Vec<String>, _> = stmt.query_map([], |row| row.get(0)).unwrap().collect();

        println!("Rows: {:?}", rows);

        match rows {
            Ok(rows) => Ok(rows),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Removes a row from a table.
    ///
    /// # Arguments
    ///
    /// * `table_name` - A string slice that holds the name of the table.
    /// * `col_name` - A string slice that holds the name of the column.
    /// * `row_id` - The id of the row to be removed.
    ///
    /// # Returns
    ///
    /// * `Result<String, String>` - The result of the row removal operation.
    fn remove_row(
        &mut self,
        table_name: &str,
        col_name: &str,
        row_id: i64,
    ) -> Result<String, String> {
        let sql = format!("DELETE FROM {} WHERE {} = {}", table_name, col_name, row_id);
        match self.conn.execute(&sql, []) {
            Ok(_) => Ok("Row removed successfully".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Inserts a row into a table.
    ///
    /// # Arguments
    ///
    /// * `table_name` - A string slice that holds the name of the table.
    /// * `row` - A vector of `SerializableValue` that represents the row to be inserted.
    ///
    /// # Returns
    ///
    /// * `Result<String, String>` - The result of the row insertion operation.
    fn insert_row(
        &mut self,
        table_name: &str,
        row: Vec<SerializableValue>,
    ) -> Result<String, String> {
        let placeholders: Vec<String> = row.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "INSERT INTO {} VALUES ({})",
            table_name,
            placeholders.join(", ")
        );

        let params: Vec<&dyn rusqlite::ToSql> = row
            .iter()
            .map(|value| match value {
                SerializableValue::Text(text) => text as &dyn rusqlite::ToSql,
                SerializableValue::Integer(int) => int as &dyn rusqlite::ToSql,
                SerializableValue::Real(real) => real as &dyn rusqlite::ToSql,
                SerializableValue::Blob(blob) => blob as &dyn rusqlite::ToSql,
                SerializableValue::Null => &rusqlite::types::Null as &dyn rusqlite::ToSql,
            })
            .collect();
        match self.conn.execute(&sql, params.as_slice()) {
            Ok(_) => Ok("Row added successfully".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Updates a row in a table.
    ///
    /// # Arguments
    ///
    /// * `table_name` - A string slice that holds the name of the table.
    /// * `col_name` - A string slice that holds the name of the column.
    /// * `index_col_name` - A string slice that holds the name of the index column.
    /// * `id` - The id of the row to be updated.
    /// * `value` - A `SerializableValue` that represents the new value.
    ///
    /// # Returns
    ///
    /// * `Result<String, String>` - The result of the row update operation.
    fn update_row(
        &mut self,
        table_name: &str,
        col_name: &str,
        index_col_name: &str,
        id: i64,
        value: SerializableValue,
    ) -> Result<String, String> {
        let sql = format!(
            "UPDATE {} SET {} = ? WHERE {} = {}",
            table_name, col_name, index_col_name, id
        );
        println!("SQL: {}", sql);
        let params: Vec<&dyn rusqlite::ToSql> = vec![match &value {
            SerializableValue::Text(text) => text as &dyn rusqlite::ToSql,
            SerializableValue::Integer(int) => int as &dyn rusqlite::ToSql,
            SerializableValue::Real(real) => real as &dyn rusqlite::ToSql,
            SerializableValue::Blob(blob) => blob as &dyn rusqlite::ToSql,
            SerializableValue::Null => &rusqlite::types::Null as &dyn rusqlite::ToSql,
        }];
        match self.conn.execute(&sql, params.as_slice()) {
            Ok(_) => Ok("Row updated successfully".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Runs a query on the database.
    ///
    /// # Arguments
    ///
    /// * `query` - A string slice that holds the query to be run.
    ///
    /// # Returns
    ///
    /// * `Result<TableRequest, String>` - The result of the query.
    fn run_query(&mut self, query: &str) -> Result<TableRequest, String> {
        let mut stmt = match self.conn.prepare(query) {
            Ok(stmt) => stmt,
            Err(e) => return Err(e.to_string()),
        };
        let total_cols = stmt.column_count();
        let rows: Result<Vec<Vec<SerializableValue>>, _> = stmt
            .query_map([], |row| {
                let mut cols = Vec::new();
                for i in 0..total_cols {
                    let value: rusqlite::types::Value = row.get(i)?;
                    cols.push(SerializableValue::from(value));
                }
                Ok(cols)
            })
            .unwrap()
            .collect();

        match rows.as_ref() {
            Ok(rows) => match rows.first() {
                Some(first_item) => {
                    let column_names: Vec<ColumnInfo> = stmt
                        .column_names()
                        .iter()
                        .zip(first_item)
                        .map(|(str, value)| ColumnInfo {
                            name: str.to_string(),
                            type_name: match value {
                                SerializableValue::Null => "NULL".to_string(),
                                SerializableValue::Integer(_) => "INTEGER".to_string(),
                                SerializableValue::Real(_) => "REAL".to_string(),
                                SerializableValue::Text(_) => "TEXT".to_string(),
                                SerializableValue::Blob(_) => "BLOB".to_string(),
                            },
                        })
                        .collect();

                    let total_rows_in_table_from_query = match self.conn.query_row(
                        &format!("SELECT COUNT(*) FROM ({})", query),
                        [],
                        |row| row.get(0),
                    ) {
                        Ok(count) => count,
                        Err(e) => return Err(e.to_string()),
                    };

                    println!(
                        "Total rows in table from query: {:?}",
                        total_rows_in_table_from_query
                    );

                    Ok(TableRequest {
                        column_names,
                        rows: rows.clone(),
                        row_count: total_rows_in_table_from_query,
                    })
                }
                None => Ok(TableRequest {
                    column_names: vec![],
                    rows: vec![],
                    row_count: 0,
                }),
            },
            Err(e) => Err(e.to_string()),
        }
    }
}
