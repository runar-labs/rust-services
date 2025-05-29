use runar_macros::service;
use runar_node::LifecycleContext;
use rusqlite::{params, Connection, OptionalExtension, Result};
use std::{collections::HashMap, sync::Arc};

/// Core value types for SQLite operations
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
    Boolean(bool),
}

/// Parameter bindings for SQL queries
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Params {
    pub values: HashMap<String, Value>,
}

impl Params {
    /// Create a new Params object
    pub fn new() -> Self {
        Self::default()
    }
    /// Add a named value
    pub fn with_value(mut self, name: &str, value: impl Into<Value>) -> Self {
        self.values.insert(name.to_string(), value.into());
        self
    }
}

/// SQL Query with typed parameters
#[derive(Debug, Clone, PartialEq)]
pub struct SqlQuery {
    pub statement: String,
    pub params: Params,
}

impl SqlQuery {
    pub fn new(statement: &str) -> Self {
        Self {
            statement: statement.to_string(),
            params: Params::new(),
        }
    }
    pub fn with_params(mut self, params: Params) -> Self {
        self.params = params;
        self
    }
}

/// Query operators for building advanced queries
#[derive(Debug, Clone, PartialEq)]
pub enum QueryOperator {
    Equal(Value),
    NotEqual(Value),
    GreaterThan(Value),
    GreaterThanOrEqual(Value),
    LessThan(Value),
    LessThanOrEqual(Value),
    Like(String),
    In(Vec<Value>),
}

/// Query builder for composable, immutable queries
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Query {
    pub conditions: HashMap<String, QueryOperator>,
}

impl Query {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_condition(mut self, field: &str, op: QueryOperator) -> Self {
        self.conditions.insert(field.to_string(), op);
        self
    }
}

/// CRUD operation types
#[derive(Debug, Clone, PartialEq)]
pub struct CreateOperation {
    pub table: String,
    pub data: HashMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReadOperation {
    pub table: String,
    pub query: Query,
    pub fields: Option<Vec<String>>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub order_by: Option<Vec<(String, bool)>>, // (field, is_ascending)
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateOperation {
    pub table: String,
    pub query: Query,
    pub updates: HashMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeleteOperation {
    pub table: String,
    pub query: Query,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CrudOperation {
    Create(CreateOperation),
    Read(ReadOperation),
    Update(UpdateOperation),
    Delete(DeleteOperation),
}

/// Schema definition for the SQLite database
#[derive(Debug, Clone, PartialEq)]
pub struct Schema {
    pub tables: Vec<TableDefinition>,
}

impl Schema {
    pub fn new() -> Self {
        Self { tables: Vec::new() }
    }
    pub fn add_table(mut self, table: TableDefinition) -> Self {
        self.tables.push(table);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableDefinition {
    pub name: String,
    pub columns: Vec<ColumnDefinition>,
    pub primary_key: Vec<String>,
    pub foreign_keys: Vec<ForeignKey>,
    pub indexes: Vec<IndexDefinition>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: DataType,
    pub constraints: Vec<ColumnConstraint>,
    pub default_value: Option<DefaultValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Integer,
    Text,
    Real,
    Blob,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColumnConstraint {
    PrimaryKey,
    NotNull,
    Unique,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DefaultValue {
    Integer(i64),
    Text(String),
    Real(f64),
    Null,
    CurrentTimestamp,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForeignKey {
    pub column: String,
    pub foreign_table: String,
    pub foreign_column: String,
    pub on_delete: ForeignKeyAction,
    pub on_update: ForeignKeyAction,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ForeignKeyAction {
    NoAction,
    Cascade,
    SetNull,
    SetDefault,
    Restrict,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexDefinition {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
}

// Intention: Implementation logic, connection pooling, and service trait integration will be added only after tests and documentation are aligned with this API.

/// SQLite Service configuration
#[derive(Debug, Clone, PartialEq)]
pub struct SqliteConfig {
    /// Path to the SQLite database file
    pub db_path: String,
    /// Schema definition for the database
    pub schema: Schema,
}

impl SqliteConfig {
    /// Create a new SQLite config with path and schema
    pub fn new(db_path: impl Into<String>, schema: Schema) -> Self {
        Self {
            db_path: db_path.into(),
            schema,
        }
    }
}

#[service(name = "sqlite", version = "1.0.0", description = "SQLite Service")]
pub struct SqliteService {
    config: SqliteConfig,
    connection: Option<Arc<Connection>>,
}

impl SqliteService {
    /// Create a new SQLite service with the given config
    pub fn new(config: SqliteConfig) -> Self {
        Self {
            config,
            connection: None,
        }
    }

    async fn start(&self, context: LifecycleContext) -> Result<()> {
        let path = self.path();
        context.info(format!("starting sqlite service at path: {}", path));
        let connection = Connection::open(self.config.db_path).unwrap();
        self.connection = Some(Arc::new(connection));
        self.initialize_schema()?;
        Ok(())
    }

    async fn stop(&self, context: LifecycleContext) -> Result<()> {
        context.info("MathService stopped".to_string());
        Ok(())
    }

    fn initialize_schema(&self, conn: &Connection) -> Result<()> {
        //
        let schema = self.config.schema;
        for table in schema.tables {
            let sql = format!(
                "CREATE TABLE {} ({});",
                table.name,
                table.columns.join(", ")
            );
            conn.execute(sql.as_str(), params![])?;
        }
        Ok(())
    }

    async fn execute_sql(&self, query: SqlQuery) -> anyhow::Result<Vec<HashMap<String, Value>>> {
        let connection = self.connection.as_ref().unwrap();
        let mut conn = connection.lock().unwrap();
        let rows = conn.query_map(query.statement, |row| {
            let mut map = HashMap::new();
            map.insert("id".to_string(), Value::Integer(row.get(0).unwrap()));
            map.insert("name".to_string(), Value::Text(row.get(1).unwrap()));
            map.insert("email".to_string(), Value::Text(row.get(2).unwrap()));
            map.insert("age".to_string(), Value::Integer(row.get(3).unwrap()));
            map
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Perform a CRUD operation (type-safe API)
    async fn execute_crud(&self, op: CrudOperation) -> anyhow::Result<Vec<HashMap<String, Value>>> {
    }
}
