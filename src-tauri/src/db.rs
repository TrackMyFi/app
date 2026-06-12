use libsql::{Builder, Connection, Database};
use tauri::{AppHandle, Manager};

pub struct Db(pub Database);

impl Db {
    pub async fn conn(&self) -> Result<Connection, String> {
        self.0.connect().map_err(|e| e.to_string())
    }
}

pub async fn init(app: &AppHandle) -> Result<Db, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("trackmyfi.db");
    let db = Builder::new_local(path)
        .build()
        .await
        .map_err(|e| e.to_string())?;
    let conn = db.connect().map_err(|e| e.to_string())?;
    crate::migrations::run(&conn).await?;
    Ok(Db(db))
}
