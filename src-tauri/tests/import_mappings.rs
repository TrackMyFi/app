use libsql::Builder;
use trackmyfi_app_lib::commands::import_mappings::{self, NewImportMapping};
use trackmyfi_app_lib::migrations;

#[tokio::test]
async fn import_mapping_roundtrip() {
    let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();
    migrations::run(&conn).await.unwrap();

    let id = import_mappings::create_import_mapping(
        &conn,
        &NewImportMapping {
            name: "Chase Checking".into(),
            config: "{\"date\":\"Posting Date\"}".into(),
            created_at: "2026-03-01".into(),
        },
    )
    .await
    .unwrap();
    assert!(id >= 1);

    let list = import_mappings::list_import_mappings(&conn).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "Chase Checking");
    assert_eq!(list[0].config, "{\"date\":\"Posting Date\"}");

    import_mappings::delete_import_mapping(&conn, id).await.unwrap();
    assert_eq!(import_mappings::list_import_mappings(&conn).await.unwrap().len(), 0);
}
