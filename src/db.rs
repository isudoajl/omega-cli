//! SQLite institutional memory database manager.
//!
//! Handles initialization, schema deployment, and migration of the
//! `.claude/memory.db` database that accumulates knowledge across
//! workflow sessions. Also deploys SQL query reference files.

use std::fs;
use std::path::Path;

use rusqlite::Connection;

/// Result of a database initialization operation.
#[derive(Debug, PartialEq, Eq)]
pub enum DbResult {
    /// Database was newly created with the given table and view counts.
    Created { tables: usize, views: usize },
    /// Database existed and was migrated (new tables/views added).
    Migrated { tables: usize, views: usize },
    /// Database existed and schema was already current.
    AlreadyCurrent { tables: usize, views: usize },
}

/// Errors that can occur during database operations.
#[derive(thiserror::Error, Debug)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Initialize (or migrate) the institutional memory database.
///
/// Creates `.claude/memory.db` inside `target_dir`, executes the full
/// schema (all statements use `CREATE IF NOT EXISTS`), and returns a
/// result indicating whether the database was created, migrated, or
/// was already current.
pub fn initialize_db(target_dir: &Path) -> Result<DbResult, DbError> {
    let claude_dir = target_dir.join(".claude");
    fs::create_dir_all(&claude_dir)?;

    let db_path = claude_dir.join("memory.db");
    let was_new = !db_path.exists();

    let conn = Connection::open(&db_path)?;

    // Enable WAL mode for better concurrent access and foreign keys.
    conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;

    // Snapshot table+view counts before applying schema (for migration detection).
    let (tables_before, views_before) = if was_new {
        (0usize, 0usize)
    } else {
        (count_tables(&conn)?, count_views(&conn)?)
    };

    // Execute the full schema. All CREATE statements are idempotent.
    let schema = crate::assets::schema_sql();
    conn.execute_batch(schema)?;

    let tables = count_tables(&conn)?;
    let views = count_views(&conn)?;

    if was_new {
        Ok(DbResult::Created { tables, views })
    } else if tables != tables_before || views != views_before {
        Ok(DbResult::Migrated { tables, views })
    } else {
        Ok(DbResult::AlreadyCurrent { tables, views })
    }
}

/// Deploy SQL query reference files to `.claude/db-queries/`.
///
/// Writes each query template (briefing, debrief, maintenance) so that
/// agents can reference them during workflow sessions.
#[allow(dead_code)]
pub fn deploy_query_files(target_dir: &Path) -> Result<(), std::io::Error> {
    let queries_dir = target_dir.join(".claude/db-queries");
    fs::create_dir_all(&queries_dir)?;

    for asset in crate::assets::query_files() {
        fs::write(queries_dir.join(asset.name), asset.content)?;
    }

    Ok(())
}

/// Count user tables (excluding internal sqlite_* tables).
fn count_tables(conn: &Connection) -> Result<usize, rusqlite::Error> {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
        [],
        |row| row.get::<_, i64>(0).map(|n| n as usize),
    )
}

/// Count views in the database.
fn count_views(conn: &Connection) -> Result<usize, rusqlite::Error> {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='view'",
        [],
        |row| row.get::<_, i64>(0).map(|n| n as usize),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_dir(label: &str) -> PathBuf {
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "omg-db-test-{}-{}-{}",
            std::process::id(),
            label,
            id,
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_initialize_db_creates_new() {
        let dir = temp_dir("creates-new");
        let result = initialize_db(&dir).unwrap();
        match result {
            DbResult::Created { tables, views: _ } => {
                assert!(tables > 0, "Expected at least one table");
            }
            other => panic!("Expected Created, got {:?}", other),
        }
        assert!(dir.join(".claude/memory.db").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_initialize_db_already_current() {
        let dir = temp_dir("already-current");
        // First init creates the DB.
        initialize_db(&dir).unwrap();
        // Second init should find it already current.
        let result = initialize_db(&dir).unwrap();
        assert!(
            matches!(result, DbResult::AlreadyCurrent { .. }),
            "Expected AlreadyCurrent, got {:?}",
            result
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_deploy_query_files() {
        let dir = temp_dir("query-files");
        deploy_query_files(&dir).unwrap();
        let queries_dir = dir.join(".claude/db-queries");
        assert!(queries_dir.exists());
        assert!(queries_dir.join("briefing.sql").exists());
        assert!(queries_dir.join("debrief.sql").exists());
        assert!(queries_dir.join("maintenance.sql").exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
