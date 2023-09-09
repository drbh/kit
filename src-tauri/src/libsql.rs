/// The `rusqlite::Result` type.
use rusqlite::Result;

/// The `DbManagerTrait` trait from the `db_manager` module.
use crate::db_manager::DbManagerTrait;
/// The `ColumnInfo` struct.
use crate::ColumnInfo;
/// The `SerializableValue` enum.
use crate::SerializableValue;
/// The `TableRequest` struct.
use crate::TableRequest;

/// The `LibsqlDbManager` struct, which represents a connection to a SQLite database.
pub struct LibsqlDbManager {
    /// The SQLite connection.
    libsqlite_conn: libsql_client::SyncClient,
}

/// Implementation of `LibsqlDbManager`.
impl LibsqlDbManager {
    /// Creates a new `LibsqlDbManager`.
    ///
    /// # Arguments
    ///
    /// * `lsql` - A `libsql_client::SyncClient` representing the SQLite connection.
    ///
    /// # Returns
    ///
    /// * `LibsqlDbManager` - The new `LibsqlDbManager`.
    pub fn new(lsql: libsql_client::SyncClient) -> Self {
        LibsqlDbManager {
            libsqlite_conn: lsql,
        }
    }
}

/// Implementation of `DbManagerTrait` for `LibsqlDbManager`.
impl DbManagerTrait for LibsqlDbManager {
    /// Gets all table names from the SQLite database.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<String>, String>` - A `Result` containing a `Vec` of table names if successful, or an error message if not.
    fn get_all_tables(&mut self) -> Result<Vec<String>, String> {
        let mut results = Vec::new();
        let query = "SELECT name FROM sqlite_master WHERE type='table'";
        let result = self.libsqlite_conn.execute(query);
        match result {
            Ok(data) => {
                for row in data.rows {
                    let mut table_name = row.value_map.get("name").unwrap().to_string();
                    // trim quotes
                    table_name = table_name.trim_matches('\"').to_string();
                    // if starts with _ skip
                    if table_name.starts_with('_') {
                        continue;
                    }
                    results.push(table_name);
                }
            }
            Err(e) => return Err(e.to_string()),
        }
        println!("Rows: {:?}", results);

        Ok(results)
    }

    /// Gets data from a specific table in the SQLite database.
    ///
    /// # Arguments
    ///
    /// * `table_name` - The name of the table.
    ///
    /// # Returns
    ///
    /// * `Result<TableRequest, String>` - A `Result` containing a `TableRequest` if successful, or an error message if not.
    fn get_table_data(&mut self, table_name: &str) -> Result<TableRequest, String> {
        println!("Getting libsql table data for: {:?}", table_name);
        let query = format!("SELECT * FROM {}", table_name);
        let result = self.libsqlite_conn.execute(query);
        match result {
            Ok(data) => {
                let mut column_names = Vec::new();
                let mut rows = Vec::new();
                if let Some(first_row) = data.rows.first() {
                    column_names = first_row
                        .value_map
                        .keys()
                        .map(|key| ColumnInfo {
                            name: key.to_string(),
                            type_name: "TEXT".to_string(),
                        })
                        .collect();
                }
                for row in data.rows {
                    let mut td = Vec::new();
                    for col in &column_names {
                        let value = row.value_map.get(&col.name).unwrap();
                        td.push(SerializableValue::Text(value.to_string()));
                    }
                    rows.push(td);
                }

                // ensure that id column is always first
                let mut id_column_index = 0;
                for (i, col) in column_names.iter().enumerate() {
                    if col.name == "id" {
                        id_column_index = i;
                        break;
                    }
                }

                if id_column_index != 0 {
                    let id_column = column_names.remove(id_column_index);
                    column_names.insert(0, id_column);
                    for row in &mut rows {
                        let id_value = row.remove(id_column_index);
                        row.insert(0, id_value);
                    }
                }

                Ok(TableRequest {
                    column_names,
                    rows,
                    row_count: 0,
                })
            }
            Err(e) => Err(e.to_string()),
        }
    }

    /// Removes a specific row from a table in the SQLite database.
    ///
    /// # Arguments
    ///
    /// * `table_name` - The name of the table.
    /// * `col_name` - The name of the column.
    /// * `row_id` - The ID of the row.
    ///
    /// # Returns
    ///
    /// * `Result<String, String>` - A `Result` containing a success message if successful, or an error message if not.
    fn remove_row(
        &mut self,
        table_name: &str,
        col_name: &str,
        row_id: i64,
    ) -> Result<String, String> {
        let sql = format!("DELETE FROM {} WHERE {} = {}", table_name, col_name, row_id);
        match self.libsqlite_conn.execute(sql) {
            Ok(_) => Ok("Row removed successfully".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Inserts a new row into a table in the SQLite database.
    ///
    /// # Arguments
    ///
    /// * `table_name` - The name of the table.
    /// * `row` - A `Vec` of `SerializableValue`s representing the row.
    ///
    /// # Returns
    ///
    /// * `Result<String, String>` - A `Result` containing a success message if successful, or an error message if not.
    fn insert_row(
        &mut self,
        table_name: &str,
        row: Vec<SerializableValue>,
    ) -> Result<String, String> {
        let mut sql = format!("INSERT INTO {} VALUES (", table_name);
        for (i, value) in row.iter().enumerate() {
            match value {
                SerializableValue::Text(text) => {
                    sql.push_str(&format!("'{}'", text));
                }
                SerializableValue::Blob(_blob) => {
                    sql.push_str(&format!("'{}'", "blob"));
                }
                SerializableValue::Null => {
                    sql.push_str(&format!("'{}'", "null"));
                }
                SerializableValue::Integer(int) => {
                    sql.push_str(&format!("{}", int));
                }
                SerializableValue::Real(real) => {
                    sql.push_str(&format!("{}", real));
                }
            }
            if i < row.len() - 1 {
                sql.push_str(", ");
            }
        }
        sql.push(')');
        match self.libsqlite_conn.execute(sql) {
            Ok(_) => Ok("Row inserted successfully".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Updates a specific row in a table in the SQLite database.
    ///
    /// # Arguments
    ///
    /// * `table_name` - The name of the table.
    /// * `col_name` - The name of the column.
    /// * `index_col_name` - The name of the index column.
    /// * `id` - The ID of the row.
    /// * `value` - A `SerializableValue` representing the new value.
    ///
    /// # Returns
    ///
    /// * `Result<String, String>` - A `Result` containing a success message if successful, or an error message if not.
    fn update_row(
        &mut self,
        table_name: &str,
        col_name: &str,
        _index_col_name: &str,
        id: i64,
        value: SerializableValue,
    ) -> Result<String, String> {
        let mut sql = format!("UPDATE {} SET ", table_name);
        match value {
            SerializableValue::Text(text) => {
                sql.push_str(&format!("{} = '{}'", col_name, text));
            }
            SerializableValue::Blob(_blob) => {
                sql.push_str(&format!("{} = '{}'", col_name, "blob"));
            }
            SerializableValue::Null => {
                sql.push_str(&format!("{} = '{}'", col_name, "null"));
            }
            SerializableValue::Integer(int) => {
                sql.push_str(&format!("{} = {}", col_name, int));
            }
            SerializableValue::Real(real) => {
                sql.push_str(&format!("{} = {}", col_name, real));
            }
        }
        sql.push_str(&format!(" WHERE id = {}", id));
        match self.libsqlite_conn.execute(sql) {
            Ok(_) => Ok("Row updated successfully".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }
}
