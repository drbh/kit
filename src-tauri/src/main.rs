use db_manager::DbManager;
use rusqlite::{types::Value, Result};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, sync::Mutex};
use tauri::{Manager, PhysicalSize, Size, State};
use window_shadows::set_shadow;

mod db_manager;
mod libsql;
mod native;

/// SerializableValue is an enum that represents a value that can be serialized.
/// It can be one of five types: Null, Integer, Real, Text, or Blob.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SerializableValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

/// This implementation allows for conversion from a Value to a SerializableValue.
impl From<Value> for SerializableValue {
    fn from(value: Value) -> Self {
        match value {
            Value::Null => SerializableValue::Null,
            Value::Integer(i) => SerializableValue::Integer(i),
            Value::Real(f) => SerializableValue::Real(f),
            Value::Text(s) => SerializableValue::Text(s),
            Value::Blob(b) => SerializableValue::Blob(b),
        }
    }
}

/// ColumnInfo is a struct that represents information about a column in a database.
/// It contains the name of the column and the type of the column.
#[derive(Serialize, Debug, PartialEq, Eq, Hash, Clone)]
struct ColumnInfo {
    name: String,
    type_name: String,
}

/// ConnectionResponse is a struct that represents the response from a connection to a database.
/// It contains a list of tables, a list of column names, a list of preview rows, and a row count.
#[derive(Serialize, Debug)]
struct ConnectionResponse {
    tables: Vec<String>,
    column_names: Vec<ColumnInfo>,
    preview_rows: Vec<Vec<SerializableValue>>,
    row_count: i64,
}

/// This implementation allows for the creation of a default ConnectionResponse.
impl Default for ConnectionResponse {
    fn default() -> Self {
        ConnectionResponse {
            tables: vec![],
            column_names: vec![],
            preview_rows: vec![],
            row_count: 0,
        }
    }
}

/// TableRequest is a struct that represents a request for a table from a database.
/// It contains a list of column names, a list of rows, and a row count.
#[derive(Serialize)]
pub struct TableRequest {
    column_names: Vec<ColumnInfo>,
    rows: Vec<Vec<SerializableValue>>,
    row_count: i64,
}

/// AppState is a struct that represents the state of the application.
/// It contains a database manager and a list of callbacks.
struct AppState {
    db: Mutex<DbManager>,
    callbacks: Arc<Mutex<HashMap<String, Box<dyn FnMut(String) + Send>>>>,
}

/// Connects to the database at the given path and returns a `ConnectionResponse`.
///
/// This function locks the `AppState`'s database manager and attempts to connect to the database.
/// If the connection is successful, it fetches all tables from the database and populates the `ConnectionResponse`
/// with the data from the first table, if any.
///
/// # Arguments
///
/// * `path` - A string slice that holds the path to the database.
/// * `state` - The `AppState` containing the database manager.
///
/// # Returns
///
/// * `Ok(ConnectionResponse)` - If the connection is successful.
/// * `Err(String)` - If the connection fails, with the error message.
#[tauri::command]
fn connect_to_db(path: String, state: State<'_, AppState>) -> Result<ConnectionResponse, String> {
    let mut db_manager: std::sync::MutexGuard<'_, DbManager> = state.db.lock().unwrap();
    match db_manager.connect_to_db(&path) {
        Ok(_) => {
            let tables = db_manager.get_all_tables()?;
            let mut response = ConnectionResponse::default();
            if !tables.is_empty() {
                response.tables = tables.clone();
                let table_data = db_manager.get_table_data(&tables[0])?;
                response.column_names = table_data.column_names;
                response.preview_rows = table_data.rows;
                response.row_count = table_data.row_count;
            }
            Ok(response)
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Fetches data from the specified table and returns a `TableRequest`.
///
/// This function locks the `AppState`'s database manager and fetches data from the specified table.
///
/// # Arguments
///
/// * `table_name` - The name of the table to fetch data from.
/// * `state` - The `AppState` containing the database manager.
///
/// # Returns
///
/// * `Ok(TableRequest)` - If the data fetch is successful.
/// * `Err(String)` - If the data fetch fails, with the error message.
#[tauri::command]
fn get_table_data(table_name: String, state: State<'_, AppState>) -> Result<TableRequest, String> {
    let mut db_manager = state.db.lock().unwrap();
    db_manager.get_table_data(&table_name)
}

/// Removes a row from the specified table.
///
/// This function locks the `AppState`'s database manager and removes a row from the specified table.
///
/// # Arguments
///
/// * `table_name` - The name of the table to remove a row from.
/// * `row_id` - The ID of the row to remove.
/// * `col_name` - The name of the column where the row is located.
/// * `state` - The `AppState` containing the database manager.
///
/// # Returns
///
/// * `Ok(String)` - If the row removal is successful.
/// * `Err(String)` - If the row removal fails, with the error message.
#[tauri::command]
fn remove_row(
    table_name: String,
    row_id: i64,
    col_name: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let mut db_manager = state.db.lock().unwrap();
    db_manager.remove_row(&table_name, &col_name, row_id)
}

/// Inserts a row into the specified table.
///
/// This function locks the `AppState`'s database manager and inserts a row into the specified table.
///
/// # Arguments
///
/// * `table_name` - The name of the table to insert a row into.
/// * `row` - The row data to insert, represented as a vector of `SerializableValue`.
/// * `state` - The `AppState` containing the database manager.
///
/// # Returns
///
/// * `Ok(String)` - If the row insertion is successful.
/// * `Err(String)` - If the row insertion fails, with the error message.
#[tauri::command]
fn insert_row(
    table_name: String,
    row: Vec<SerializableValue>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let mut db_manager = state.db.lock().unwrap();
    db_manager.insert_row(&table_name, row)
}

/// Updates a row in the specified table.
///
/// This function locks the `AppState`'s database manager and updates a row in the specified table.
///
/// # Arguments
///
/// * `table_name` - The name of the table to update a row in.
/// * `col_name` - The name of the column where the row is located.
/// * `index_col_name` - The name of the index column.
/// * `id` - The ID of the row to update.
/// * `value` - The new value to update the row with.
/// * `state` - The `AppState` containing the database manager.
///
/// # Returns
///
/// * `Ok(String)` - If the row update is successful.
/// * `Err(String)` - If the row update fails, with the error message.
#[tauri::command]
fn update_row(
    table_name: String,
    col_name: String,
    index_col_name: String,
    id: i64,
    value: SerializableValue,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let mut db_manager = state.db.lock().unwrap();
    db_manager.update_row(&table_name, &col_name, &index_col_name, id, value)
}

/// Runs a query on the database.
///
/// This function locks the `AppState`'s database manager and runs a query on the database.
///
/// # Arguments
///
/// * `query` - The query to run on the database.
/// * `state` - The `AppState` containing the database manager.
///     
/// # Returns
///
/// * `Ok(TableRequest)` - If the query is successful.
/// * `Err(String)` - If the query fails, with the error message.
#[tauri::command]
fn sql_query(query: String, state: State<'_, AppState>) -> Result<TableRequest, String> {
    let mut db_manager = state.db.lock().unwrap();
    db_manager.run_query(&query)
}

/// Subscribes to changes in the database.
///
/// This function takes a callback function as an argument.
/// The callback function is invoked whenever a change is detected in the database.
#[tauri::command]
fn subscribe(
    handler: usize,
    function_name: String,
    _state: State<'_, AppState>,
    window: tauri::Window,
) {
    let msg = "dummy".to_string();
    window
        .eval(&format!("window._{}({:?})", handler, msg))
        .unwrap();
    println!("[callback] {} invoked", function_name);
}

/// Registers a callback function.
///
/// This function takes a callback function as an argument and stores it in the application state.
/// The callback function is invoked whenever a change is detected in the database.
///
/// # Arguments
///
/// * `handler` - The handler ID for the callback function.
/// * `function_name` - The name of the callback function.
/// * `state` - The `AppState` containing the database manager.
/// * `window` - The `tauri::Window` object representing the application window.
#[tauri::command]
fn register_callback(
    handler: usize,
    function_name: String,
    state: State<'_, AppState>,
    window: tauri::Window,
) {
    let mut callbacks = state.callbacks.lock().unwrap();
    let callback = move |msg: String| {
        window
            .eval(&format!("window._{}({:?})", handler, msg))
            .unwrap();
    };
    callbacks.insert(function_name.to_string(), Box::new(callback));
}

/// Returns the current time in seconds since the UNIX epoch.
///
/// # Returns
///
/// * `u64` - The current time in seconds since the UNIX epoch.
fn current_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Periodically invokes the registered callback functions.
///
/// This function is an asynchronous function that runs in an infinite loop.
/// It invokes the registered callback functions every `seconds` seconds.
///
/// # Arguments
///
/// * `seconds` - The interval in seconds at which to invoke the callback functions.
/// * `callback_mutex_clone` - A clone of the mutex protecting the callback functions.
/// * `is_premium` - A boolean indicating whether the user has a premium subscription.
async fn periodic_callback(
    seconds: u64,
    callback_mutex_clone: Arc<Mutex<HashMap<String, Box<dyn FnMut(String) + Send>>>>,
    is_premium: bool,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(seconds));
    let call_back_mutex_clone = callback_mutex_clone.clone();
    let last_shown = Arc::new(Mutex::new(current_time()));
    loop {
        interval.tick().await;
        let mut callbacks = call_back_mutex_clone.lock().unwrap();
        for (key, callback) in callbacks.iter_mut() {
            if key == "macAddress" {
                let mac_address = mac_address::get_mac_address().unwrap().unwrap();
                callback(mac_address.to_string());
            }
            if key == "freeTrialPopup" {
                let current_time = current_time();
                let condition_every_tenth = current_time % 60 == 10;
                let condition_every_thirtieth = current_time % 60 == 30;
                let condition_every_fiftieth = current_time % 60 == 50;
                if !is_premium
                    && (condition_every_tenth
                        || condition_every_thirtieth
                        || condition_every_fiftieth)
                {
                    callback(current_time.to_string());
                    *last_shown.lock().unwrap() = current_time;
                }
            }
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub user_mac_address: UserMacAddress,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserMacAddress {
    pub columns: Vec<String>,
    pub rows: Vec<serde_json::Value>,
    pub rows_affected: i64,
    pub last_insert_rowid: Option<usize>,
}

/// Checks if the user has a premium subscription.
///
/// This function sends a GET request to a server with the MAC address of the machine.
/// If the server responds with a success status, the user is considered to have a premium subscription.
///
/// # Returns
///
/// * `Result<bool, Box<dyn Error>>` - A `Result` containing a boolean indicating whether the user has a premium subscription.
async fn check_if_premium() -> Result<bool, Box<dyn Error>> {
    let mac_address_of_this_machine = mac_address::get_mac_address()?.unwrap();
    let url = format!(
        "https://kit-services.drbh.workers.dev/registration?mac_address={}",
        mac_address_of_this_machine
    );

    let response = reqwest::get(&url).await?;

    if !response.status().is_success() {
        println!("Failed to register MAC address");
        return Ok(false);
    }

    println!("MAC address registered successfully");

    // Parse the body of the response
    let body: Root = response.json().await?;

    let mac_address = body
        .user_mac_address
        .rows
        .get(0)
        .ok_or("No rows in response")?
        .get(1)
        .ok_or("No second column in row")?
        .as_str()
        .ok_or("Value not a string")?
        .to_string();

    if mac_address != mac_address_of_this_machine.to_string() {
        println!("MAC address verification failed");
        return Ok(false);
    }

    println!("MAC address verified successfully");
    Ok(true)
}

#[tokio::main]
async fn main() {
    #[cfg(any(windows, target_os = "macos"))]
    let app_state = AppState {
        db: Mutex::new(DbManager::new()),
        callbacks: Arc::new(Mutex::new(HashMap::new())),
    };

    let is_premium = check_if_premium().await.unwrap_or(false);
    let callback_mutex_clone = app_state.callbacks.clone();
    tokio::spawn(async move {
        periodic_callback(1, callback_mutex_clone, is_premium).await;
    });

    tauri::Builder::default()
        .setup(|app| {
            let window = app.get_window("main").unwrap();
            window
                .set_decorations(false)
                .expect("Unsupported platform!");
            window.set_resizable(false).expect("Unsupported platform!");

            window
                .set_size(Size::Physical(PhysicalSize {
                    width: 2400,
                    height: 1650,
                }))
                .unwrap();

            set_shadow(&window, true).expect("Unsupported platform!");
            Ok(())
        })
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            connect_to_db,
            get_table_data,
            remove_row,
            insert_row,
            update_row,
            subscribe,
            register_callback,
            sql_query
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
