use crate::native::NativeDbManager;
use crate::TableRequest;
use crate::{libsql::LibsqlDbManager, SerializableValue};
use rusqlite::{Connection, Result};

/// `ConnectionType` is an enum that represents the type of database connection.
/// It can be one of two types: `Sqlite` or `Libsql`.
#[derive(Debug, Clone)]
pub enum ConnectionType {
    /// `Sqlite` variant takes a string which represents the path to the sqlite database.
    Sqlite(String),
    /// `Libsql` variant takes two strings which represent the host and token for the libsql database.
    Libsql(String, String),
}

/// `DbManager` is a struct that holds a database manager.
/// The database manager is a trait object that implements the `DbManagerTrait`.
pub struct DbManager {
    /// `db` is a Box holding a trait object that implements `DbManagerTrait` and `Send`.
    pub db: Box<dyn DbManagerTrait + Send>,
}

/// `DbManagerTrait` is a trait that defines the operations that a database manager should support.
pub trait DbManagerTrait {
    /// `get_all_tables` is a method that returns all table names in the database.
    fn get_all_tables(&mut self) -> Result<Vec<String>, String>;
    /// `get_table_data` is a method that returns the data of a specific table.
    fn get_table_data(&mut self, table_name: &str) -> Result<TableRequest, String>;
    /// `remove_row` is a method that removes a specific row from a table.
    fn remove_row(
        &mut self,
        table_name: &str,
        col_name: &str,
        row_id: i64,
    ) -> Result<String, String>;
    /// `insert_row` is a method that inserts a new row into a table.
    fn insert_row(
        &mut self,
        table_name: &str,
        row: Vec<SerializableValue>,
    ) -> Result<String, String>;
    /// `update_row` is a method that updates a specific row in a table.
    fn update_row(
        &mut self,
        table_name: &str,
        col_name: &str,
        index_col_name: &str,
        id: i64,
        value: SerializableValue,
    ) -> Result<String, String>;
    /// `run_query` is a method that runs a query on the database.
    fn run_query(&mut self, query: &str) -> Result<TableRequest, String>;
}

/// `DbManager` implementation.
impl DbManager {
    /// Creates a new `DbManager` with a native database manager.
    pub fn new() -> Self {
        DbManager {
            db: Box::new(NativeDbManager::new(Connection::open(":memory:").unwrap())),
        }
    }

    /// Connects to a database given a path.
    /// The path can be a `libsql` URL or a local file path.
    pub fn connect_to_db(&mut self, path: &str) -> Result<usize, String> {
        // check if starts with libsql:// or / or it's a relative path
        let connection_type = if path.starts_with("libsql://") {
            ConnectionType::Libsql(
                path.split("::").next().unwrap().to_string(),
                path.split("::").nth(1).unwrap().to_string(),
            )
        } else if path.starts_with('/') {
            ConnectionType::Sqlite(path.to_string())
        } else {
            ConnectionType::Sqlite(format!(
                "{}/../{}",
                std::env::current_dir().unwrap().display(),
                path
            ))
        };

        println!("Connecting to: {:?}", connection_type);

        match connection_type {
            ConnectionType::Sqlite(path) => {
                self.db = Box::new(NativeDbManager::new(
                    Connection::open_with_flags(
                        path,
                        // dont create file if it doesn't exist
                        // rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
                        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
                    )
                    .unwrap_or(Connection::open_in_memory().unwrap()),
                ));
                Ok(1)
            }
            ConnectionType::Libsql(host, token) => {
                println!("Connecting to libsql: {:?} {:?}", host, token);
                self.db = Box::new(LibsqlDbManager::new(
                    libsql_client::SyncClient::from_config(libsql_client::Config {
                        url: host.as_str().try_into().unwrap(),
                        auth_token: Some(token),
                    })
                    .unwrap(),
                ));
                Ok(1)
            }
        }
    }

    /// Fetches the data of a specific table.
    pub fn get_table_data(&mut self, table_name: &str) -> Result<TableRequest, String> {
        println!("Getting table data for: {:?}", table_name);
        self.db.get_table_data(table_name)
    }

    /// Fetches all table names in the database.
    pub fn get_all_tables(&mut self) -> Result<Vec<String>, String> {
        self.db.get_all_tables()
    }

    /// Removes a specific row from a table.
    pub fn remove_row(
        &mut self,
        table_name: &str,
        col_name: &str,
        row_id: i64,
    ) -> Result<String, String> {
        self.db.remove_row(table_name, col_name, row_id)
    }

    /// Inserts a new row into a table.
    pub fn insert_row(
        &mut self,
        table_name: &str,
        row: Vec<SerializableValue>,
    ) -> Result<String, String> {
        self.db.insert_row(table_name, row)
    }

    /// Updates a specific row in a table.
    pub fn update_row(
        &mut self,
        table_name: &str,
        col_name: &str,
        index_col_name: &str,
        id: i64,
        row: SerializableValue,
    ) -> Result<String, String> {
        self.db
            .update_row(table_name, col_name, index_col_name, id, row)
    }

    /// Runs a query on the database.
    pub fn run_query(&mut self, query: &str) -> Result<TableRequest, String> {
        self.db.run_query(query)
    }
}
