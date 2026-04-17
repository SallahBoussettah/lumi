//! Manages the Kokoro TTS Python sidecar process.
//!
//! In dev, spawns `uv run python server.py` from src-tauri/sidecar/tts/.
//! In production we'll need to bundle a packaged interpreter — TODO.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const TTS_PORT: u16 = 17891;
pub const TTS_BASE_URL: &str = "http://127.0.0.1:17891";

pub struct TtsSidecar {
    child: Mutex<Option<Child>>,
}

impl TtsSidecar {
    pub fn new() -> Self {
        Self {
            child: Mutex::new(None),
        }
    }

    fn sidecar_dir() -> PathBuf {
        // CARGO_MANIFEST_DIR points at src-tauri/. The sidecar lives at
        // src-tauri/sidecar/tts.
        let manifest = env!("CARGO_MANIFEST_DIR");
        PathBuf::from(manifest).join("sidecar").join("tts")
    }

    /// Spawn the sidecar if it isn't already running.
    /// Returns immediately — caller can poll /health to know when it's ready.
    pub fn start(&self) -> Result<(), String> {
        let mut guard = self.child.lock().map_err(|e| e.to_string())?;
        if guard.is_some() {
            return Ok(());
        }

        // If something else is already serving the port (e.g. user ran the
        // sidecar manually), don't fight over it — just don't spawn.
        if reachable_quick() {
            log::info!("TTS sidecar already running on {}", TTS_BASE_URL);
            return Ok(());
        }

        let dir = Self::sidecar_dir();
        if !dir.exists() {
            return Err(format!("TTS sidecar dir not found: {}", dir.display()));
        }

        log::info!("Spawning TTS sidecar in {}", dir.display());
        let child = Command::new("uv")
            .args(["run", "python", "server.py", "--port", &TTS_PORT.to_string()])
            .current_dir(&dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn TTS sidecar: {}", e))?;

        log::info!("TTS sidecar spawned (pid {})", child.id());
        *guard = Some(child);
        Ok(())
    }

    /// Block until /health responds (or timeout). Cheap to call repeatedly.
    pub async fn wait_until_ready(&self, timeout: Duration) -> Result<(), String> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if reachable_quick() {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        Err(format!(
            "TTS sidecar didn't become ready within {:?}",
            timeout
        ))
    }

    pub fn stop(&self) {
        if let Ok(mut guard) = self.child.lock() {
            if let Some(mut child) = guard.take() {
                log::info!("Killing TTS sidecar (pid {})", child.id());
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

impl Drop for TtsSidecar {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Quick TCP probe — used to detect "already running" without async setup.
fn reachable_quick() -> bool {
    use std::net::TcpStream;
    TcpStream::connect_timeout(
        &format!("127.0.0.1:{}", TTS_PORT).parse().unwrap(),
        Duration::from_millis(150),
    )
    .is_ok()
}

// ===== HTTP client to talk to the sidecar =====

#[derive(Serialize)]
pub struct SpeakRequest<'a> {
    pub text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct WordTiming {
    pub text: String,
    pub start_ms: u32,
    pub end_ms: u32,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct SpeakResponse {
    pub text: String,
    pub audio_b64: String,
    pub sample_rate: u32,
    pub duration_ms: u32,
    pub words: Vec<WordTiming>,
}

pub async fn speak(
    http: &reqwest::Client,
    text: &str,
    voice: Option<&str>,
    speed: Option<f32>,
) -> Result<SpeakResponse, String> {
    let body = serde_json::json!({
        "text": text,
        "voice": voice.unwrap_or("af_heart"),
        "speed": speed.unwrap_or(1.0),
    });

    let resp = http
        .post(format!("{}/tts", TTS_BASE_URL))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("TTS request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let txt = resp.text().await.unwrap_or_default();
        return Err(format!("TTS error {}: {}", status, txt));
    }

    resp.json::<SpeakResponse>()
        .await
        .map_err(|e| format!("TTS bad json: {}", e))
}
