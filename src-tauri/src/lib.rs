mod audio;
mod db;

use audio::AudioState;
use cpal::Stream;
use db::Database;
use std::sync::{Arc, Mutex};

/// Wrapper to make cpal::Stream usable in Tauri state (Send + Sync)
struct StreamHolder(Mutex<Option<Stream>>);

// SAFETY: cpal::Stream on Linux (ALSA/PipeWire) is thread-safe in practice.
// The Stream is only accessed behind a Mutex.
unsafe impl Send for StreamHolder {}
unsafe impl Sync for StreamHolder {}

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

#[tauri::command]
fn list_audio_devices() -> Vec<String> {
    audio::capture::list_input_devices()
}

#[tauri::command]
fn start_recording(
    audio_state: tauri::State<'_, Arc<AudioState>>,
    stream_holder: tauri::State<'_, StreamHolder>,
) -> Result<String, String> {
    if audio_state.is_recording() {
        return Err("Already recording".to_string());
    }

    let stream = audio::capture::start_capture(audio_state.inner().clone())?;

    let mut holder = stream_holder.0.lock().map_err(|e| e.to_string())?;
    *holder = Some(stream);

    Ok("Recording started".to_string())
}

#[tauri::command]
fn stop_recording(
    audio_state: tauri::State<'_, Arc<AudioState>>,
    stream_holder: tauri::State<'_, StreamHolder>,
) -> Result<String, String> {
    let mut holder = stream_holder.0.lock().map_err(|e| e.to_string())?;
    *holder = None;
    audio_state.set_recording(false);
    audio_state.set_level(0.0);
    log::info!("Audio capture stopped");
    Ok("Recording stopped".to_string())
}

#[tauri::command]
fn get_audio_level(audio_state: tauri::State<'_, Arc<AudioState>>) -> u32 {
    audio_state.get_level()
}

#[tauri::command]
fn is_recording(audio_state: tauri::State<'_, Arc<AudioState>>) -> bool {
    audio_state.is_recording()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db_path = db::db_path();
    let database = Arc::new(Database::open(&db_path).expect("failed to open database"));
    let audio_state = Arc::new(AudioState::new());
    let stream_holder = StreamHolder(Mutex::new(None));

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
        .manage(audio_state)
        .manage(stream_holder)
        .invoke_handler(tauri::generate_handler![
            get_db_stats,
            list_audio_devices,
            start_recording,
            stop_recording,
            get_audio_level,
            is_recording,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
