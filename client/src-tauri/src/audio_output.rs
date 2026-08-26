// Linux-only output-device routing, done ourselves rather than through the
// webview: WebKitGTK has never implemented HTMLMediaElement.setSinkId, so
// there's no browser API for this here. PipeWire ships a PulseAudio-
// compatible control socket on every mainstream distro (including one this
// app never talks to directly - `pactl` does), so we shell out to that
// rather than linking libpulse ourselves. Every audio stream this app opens
// (each participant's remote audio, one per voice call peer) shows up as
// its own PulseAudio "sink input" tagged with our own PID, which is how we
// find and move just our streams without touching anyone else's audio.

use serde::Serialize;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct AudioSink {
    pub name: String,
    pub description: String,
}

fn run_pactl(args: &[&str]) -> Result<String, String> {
    let output = Command::new("pactl")
        .args(args)
        .output()
        .map_err(|_| "pactl not found - is PipeWire or PulseAudio running?".to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[tauri::command]
pub fn get_default_audio_sink() -> Result<String, String> {
    Ok(run_pactl(&["get-default-sink"])?.trim().to_string())
}

#[tauri::command]
pub fn list_audio_sinks() -> Result<Vec<AudioSink>, String> {
    let raw = run_pactl(&["-f", "json", "list", "sinks"])?;
    let value: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let sinks = value.as_array().ok_or("unexpected pactl output")?;

    Ok(sinks
        .iter()
        .filter_map(|sink| {
            let name = sink.get("name")?.as_str()?.to_string();
            let description = sink.get("description")?.as_str().unwrap_or(&name).to_string();
            Some(AudioSink { name, description })
        })
        .collect())
}

/// Moves every currently-open audio stream belonging to this process onto
/// the named sink. Safe to call with no active streams (e.g. no call
/// joined yet) - it just has nothing to move, the preference itself is
/// persisted client-side and re-applied whenever a new stream opens.
#[tauri::command]
pub fn set_app_audio_sink(sink_name: String) -> Result<(), String> {
    let pid = std::process::id().to_string();
    let raw = run_pactl(&["-f", "json", "list", "sink-inputs"])?;
    let value: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let inputs = value.as_array().ok_or("unexpected pactl output")?;

    for input in inputs {
        let Some(index) = input.get("index").and_then(|v| v.as_i64()) else {
            continue;
        };
        let owns_stream = input
            .get("properties")
            .and_then(|p| p.get("application.process.id"))
            .and_then(|v| v.as_str())
            .is_some_and(|owner_pid| owner_pid == pid);
        if !owns_stream {
            continue;
        }
        run_pactl(&["move-sink-input", &index.to_string(), &sink_name])?;
    }

    Ok(())
}
