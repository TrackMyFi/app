// Migration strategy (decided in Task 1 spike): hand-rolled ordered-SQL runner.
// Reason: neither refinery nor sqlx drives a libsql connection directly.
use libsql::Connection;

struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "init",
        sql: include_str!("../migrations/0001_init.sql"),
    },
    Migration {
        version: 2,
        name: "transactions",
        sql: include_str!("../migrations/0002_transactions.sql"),
    },
    Migration {
        version: 3,
        name: "paychecks",
        sql: include_str!("../migrations/0003_paychecks.sql"),
    },
    Migration {
        version: 4,
        name: "hsa_coverage",
        sql: include_str!("../migrations/0004_hsa_coverage.sql"),
    },
];

pub async fn run(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        (),
    )
    .await
    .map_err(|e| e.to_string())?;

    let mut applied = std::collections::HashSet::new();
    let mut rows = conn
        .query("SELECT version FROM schema_migrations", ())
        .await
        .map_err(|e| e.to_string())?;
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        applied.insert(row.get::<i64>(0).map_err(|e| e.to_string())?);
    }

    for m in MIGRATIONS {
        if applied.contains(&m.version) {
            continue;
        }
        // execute_batch returns Result<BatchRows>; bind to _ to drop it
        let _ = conn.execute_batch(m.sql).await.map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            libsql::params![m.version, m.name],
        )
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
