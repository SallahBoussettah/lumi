mod db;

use db::Database;
use std::sync::Arc;
#[tauri::command]
fn get_db_stats(db: tauri::State<'_, Arc<Database>>) -> Result<serde_json::Value, String> {
    let conn = db.conn();

    let conversations: i64 = conn
        .query_row("SELECT COUNT(*) FROM conversations", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let memories: i64 = conn
        .query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let action_items: i64 = conn
        .query_row("SELECT COUNT(*) FROM action_items", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let screenshots: i64 = conn
        .query_row("SELECT COUNT(*) FROM screenshots", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "conversations": conversations,
        "memories": memories,
        "action_items": action_items,
        "screenshots": screenshots,
    }))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db_path = db::db_path();
    log::info!("Database path: {:?}", db_path);

    let database = Arc::new(
        Database::open(&db_path).expect("failed to open database"),
    );

    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .manage(database)
        .invoke_handler(tauri::generate_handler![get_db_stats])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
