use rusqlite::{Connection, Result, params, OptionalExtension};
use tempfile::NamedTempFile;

#[derive(Debug)]
struct User {
    _id: i64,  // Prefix with underscore to indicate it's intentionally unused
    name: String,
    email: String,
    age: Option<i32>,
}

// Helper function to create an in-memory database for testing
fn create_test_db() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    initialize_schema(&conn)?;
    Ok(conn)
}

// Helper function to create a temporary file-based database
fn create_temp_db() -> Result<(Connection, NamedTempFile)> {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap();
    let conn = Connection::open(path)?;
    initialize_schema(&conn)?;
    Ok((conn, temp_file))
}

// Initialize the database schema
fn initialize_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE users (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT UNIQUE NOT NULL,
            age INTEGER
        );
        CREATE INDEX idx_users_email ON users(email);
        "#,
    )
}

#[tokio::test]
async fn test_basic_operations() {
    test_basic_operations_impl().unwrap();
}

fn test_basic_operations_impl() -> Result<()> {
    // Create an in-memory database
    let conn = create_test_db()?;

    // Insert a new user
    conn.execute(
        "INSERT INTO users (name, email, age) VALUES (?1, ?2, ?3)",
        params!["John Doe", "john@example.com", 30],
    )?;

    // Query a user
    let user = conn.query_row(
        "SELECT id, name, email, age FROM users WHERE id = ?",
        [1],
        |row| {
            Ok(User {
                _id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                age: row.get(3).ok(),
            })
        },
    )?;
    assert_eq!(user.name, "John Doe");
    assert_eq!(user.email, "john@example.com");
    assert_eq!(user.age, Some(30));

    // Update the user
    conn.execute(
        "UPDATE users SET age = ? WHERE id = ?",
        params![31, 1],
    )?;
    let updated_age: Option<i32> = conn.query_row(
        "SELECT age FROM users WHERE id = ?",
        [1],
        |row| row.get(0),
    ).optional()?;
    assert_eq!(updated_age, Some(31));

    // Delete the user
    conn.execute("DELETE FROM users WHERE id = ?", [1])?;
    let deleted: Option<User> = conn.query_row(
        "SELECT id, name, email, age FROM users WHERE id = ?",
        [1],
        |row| {
            Ok(User {
                _id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                age: row.get(3).ok(),
            })
        },
    ).optional()?;
    assert!(deleted.is_none());

    Ok(())
}
