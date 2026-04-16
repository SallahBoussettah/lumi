use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};
use std::sync::Arc;

use super::AudioState;

/// Lists available input devices and returns their names
pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|devices| {
            devices
                .filter_map(|d| d.name().ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Starts recording from the default input device.
/// Audio is captured and RMS level is updated in AudioState.
/// Returns the stream handle (must be kept alive).
pub fn start_capture(state: Arc<AudioState>) -> Result<Stream, String> {
    let host = cpal::default_host();

    let device = host
        .default_input_device()
        .ok_or("No input device available")?;

    let device_name = device.name().unwrap_or("unknown".to_string());
    log::info!("Using input device: {}", device_name);

    let config = device
        .default_input_config()
        .map_err(|e| format!("Failed to get input config: {}", e))?;

    log::info!(
        "Input config: {} channels, {}Hz, {:?}",
        config.channels(),
        config.sample_rate().0,
        config.sample_format()
    );

    let state_clone = state.clone();
    let err_fn = |err: cpal::StreamError| {
        log::error!("Audio stream error: {}", err);
    };

    let stream = match config.sample_format() {
        SampleFormat::F32 => device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    process_samples(&state_clone, data);
                },
                err_fn,
                None,
            )
            .map_err(|e| format!("Failed to build f32 stream: {}", e))?,
        SampleFormat::I16 => device
            .build_input_stream(
                &config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let floats: Vec<f32> = data.iter().map(|&s| s as f32 / 32768.0).collect();
                    process_samples(&state_clone, &floats);
                },
                err_fn,
                None,
            )
            .map_err(|e| format!("Failed to build i16 stream: {}", e))?,
        SampleFormat::I32 => device
            .build_input_stream(
                &config.into(),
                move |data: &[i32], _: &cpal::InputCallbackInfo| {
                    let floats: Vec<f32> = data.iter().map(|&s| s as f32 / 2147483648.0).collect();
                    process_samples(&state_clone, &floats);
                },
                err_fn,
                None,
            )
            .map_err(|e| format!("Failed to build i32 stream: {}", e))?,
        format => return Err(format!("Unsupported sample format: {:?}", format)),
    };

    stream
        .play()
        .map_err(|e| format!("Failed to play stream: {}", e))?;

    state.set_recording(true);
    log::info!("Audio capture started");

    Ok(stream)
}

/// Compute RMS level from audio samples and update state
fn process_samples(state: &AudioState, samples: &[f32]) {
    if samples.is_empty() {
        return;
    }

    let sum_squares: f32 = samples.iter().map(|s| s * s).sum();
    let rms = (sum_squares / samples.len() as f32).sqrt();

    // Scale RMS to a more useful range (most speech is in 0.01-0.3 range)
    let level = (rms * 5.0).clamp(0.0, 1.0);
    state.set_level(level);
}
