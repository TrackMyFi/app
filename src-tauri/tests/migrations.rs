use libsql::Builder;

#[tokio::test]
async fn migrations_create_all_tables() {
    let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();
    trackmyfi_app_lib::migrations::run(&conn).await.unwrap();

    let mut names = std::collections::HashSet::new();
    let mut rows = conn
        .query("SELECT name FROM sqlite_master WHERE type='table'", ())
        .await
        .unwrap();
    while let Some(row) = rows.next().await.unwrap() {
        names.insert(row.get::<String>(0).unwrap());
    }
    for t in [
        "fire_profile",
        "account",
        "account_balance",
        "schema_migrations",
    ] {
        assert!(names.contains(t), "missing table {t}");
    }
}
