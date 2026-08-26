// Discord-compatible Rich Presence IPC server.
//
// Games/launchers that already speak "Discord RPC" (the de-facto standard
// local-socket protocol used by discord-rpc / discord-game-sdk) connect to
// a local socket named `discord-ipc-0` (or `-1`, `-2`, ... if that index is
// taken) and send a small JSON-RPC-ish handshake followed by SET_ACTIVITY
// calls. Real Discord listens on the same name, so this server tries the
// same sequence of indices Discord itself uses and just binds whichever is
// still free - if Discord is also running, games that already found
// Discord's socket keep talking to Discord; ones that find ours (or are
// pointed at us via DISCORD_IPC_PATH type overrides) get relayed into
// HollowChat's own presence instead. The wire format is a small, publicly
// documented local-socket framing - nothing proprietary is reimplemented,
// only the open framing/opcode layout.
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use interprocess::local_socket::tokio::{prelude::*, Stream};
use interprocess::local_socket::{GenericFilePath, ListenerOptions, ToFsName};
#[cfg(windows)]
use interprocess::local_socket::{GenericNamespaced, ToNsName};
use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::timeout;

const OP_HANDSHAKE: u32 = 0;
const OP_FRAME: u32 = 1;
const OP_CLOSE: u32 = 2;
const OP_PING: u32 = 3;
const OP_PONG: u32 = 4;
const MAX_INDEX: u8 = 9;
const MAX_FRAME_BYTES: u32 = 16 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct PresenceUpdate {
    pub application_id: Option<String>,
    pub details: Option<String>,
    pub state: Option<String>,
    pub large_text: Option<String>,
    pub large_image: Option<String>,
    /// Milliseconds since the Unix epoch, straight off the wire - the
    /// activity's elapsed-time anchor (Discord RPC's `timestamps.start`).
    pub start_timestamp: Option<i64>,
}

#[cfg(unix)]
fn socket_path(index: u8) -> PathBuf {
    let dir = std::env::var("XDG_RUNTIME_DIR")
        .or_else(|_| std::env::var("TMPDIR"))
        .unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(dir).join(format!("discord-ipc-{index}"))
}

#[cfg(unix)]
fn make_name(index: u8) -> io::Result<interprocess::local_socket::Name<'static>> {
    socket_path(index)
        .to_fs_name::<GenericFilePath>()
        .map_err(|e| io::Error::other(e.to_string()))
}

#[cfg(windows)]
fn make_name(index: u8) -> io::Result<interprocess::local_socket::Name<'static>> {
    format!("discord-ipc-{index}")
        .to_ns_name::<GenericNamespaced>()
        .map_err(|e| io::Error::other(e.to_string()))
}

/// A bind attempt can fail because a *stale* socket file was left behind by
/// a previous crashed run (Unix only - Windows named pipes are cleaned up
/// by the OS when the owning process dies). Before assuming the index is
/// genuinely taken by another live server (possibly real Discord - never
/// touch that), probe it with a short-lived connection attempt: nobody
/// answering means it's safe to unlink and retry once.
#[cfg(unix)]
async fn reclaim_if_stale(index: u8) -> bool {
    let path = socket_path(index);
    let Ok(name) = make_name(index) else { return false };
    let probe = timeout(Duration::from_millis(200), Stream::connect(name)).await;
    match probe {
        Ok(Ok(_)) => false, // something real is listening - leave it alone
        _ => std::fs::remove_file(&path).is_ok(),
    }
}

#[cfg(windows)]
async fn reclaim_if_stale(_index: u8) -> bool {
    false
}

async fn bind_first_free() -> Option<interprocess::local_socket::tokio::Listener> {
    for index in 0..=MAX_INDEX {
        let Ok(name) = make_name(index) else { continue };
        match ListenerOptions::new().name(name.clone()).create_tokio() {
            Ok(listener) => {
                tracing_log(&format!("rich presence listening on discord-ipc-{index}"));
                return Some(listener);
            }
            Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
                if reclaim_if_stale(index).await {
                    if let Ok(listener) = ListenerOptions::new().name(name).create_tokio() {
                        tracing_log(&format!("rich presence reclaimed stale discord-ipc-{index}"));
                        return Some(listener);
                    }
                }
                continue;
            }
            Err(_) => continue,
        }
    }
    None
}

fn tracing_log(msg: &str) {
    #[cfg(debug_assertions)]
    eprintln!("[rpc] {msg}");
    #[cfg(not(debug_assertions))]
    let _ = msg;
}

async fn read_frame(stream: &mut Stream) -> io::Result<(u32, serde_json::Value)> {
    let mut header = [0u8; 8];
    stream.read_exact(&mut header).await?;
    let opcode = u32::from_le_bytes(header[0..4].try_into().unwrap());
    let len = u32::from_le_bytes(header[4..8].try_into().unwrap());
    if len > MAX_FRAME_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "frame too large"));
    }
    let mut body = vec![0u8; len as usize];
    stream.read_exact(&mut body).await?;
    let value = if body.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null)
    };
    Ok((opcode, value))
}

async fn write_frame(stream: &mut Stream, opcode: u32, payload: &serde_json::Value) -> io::Result<()> {
    let body = serde_json::to_vec(payload)?;
    let mut out = Vec::with_capacity(8 + body.len());
    out.extend_from_slice(&opcode.to_le_bytes());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    stream.write_all(&out).await
}

fn parse_activity(args: &serde_json::Value) -> PresenceUpdate {
    let activity = &args["activity"];
    PresenceUpdate {
        application_id: args["client_id"].as_str().map(str::to_owned),
        details: activity["details"].as_str().map(str::to_owned),
        state: activity["state"].as_str().map(str::to_owned),
        large_text: activity["assets"]["large_text"].as_str().map(str::to_owned),
        large_image: activity["assets"]["large_image"].as_str().map(str::to_owned),
        start_timestamp: activity["timestamps"]["start"].as_i64(),
    }
}

async fn handle_client<F: Fn(Option<PresenceUpdate>) + Send + 'static>(
    mut stream: Stream,
    on_update: F,
) -> io::Result<()> {
    let (op, handshake) = read_frame(&mut stream).await?;
    if op != OP_HANDSHAKE {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "expected handshake"));
    }
    let client_id: Option<String> = handshake["client_id"].as_str().map(str::to_owned);

    write_frame(
        &mut stream,
        OP_FRAME,
        &serde_json::json!({
            "cmd": "DISPATCH",
            "evt": "READY",
            "data": {
                "v": 1,
                "config": { "api_endpoint": "", "cdn_host": "", "environment": "" },
                "user": { "id": "0", "username": "HollowChat", "discriminator": "0000" }
            }
        }),
    )
    .await?;

    loop {
        let (op, frame) = match read_frame(&mut stream).await {
            Ok(v) => v,
            Err(_) => break,
        };

        match op {
            OP_FRAME => {
                let cmd = frame["cmd"].as_str().unwrap_or("");
                if cmd == "SET_ACTIVITY" {
                    let mut args = frame["args"].clone();
                    if let Some(id) = &client_id {
                        if args["client_id"].is_null() {
                            args["client_id"] = serde_json::Value::String(id.clone());
                        }
                    }
                    let has_activity = !args["activity"].is_null();
                    on_update(if has_activity { Some(parse_activity(&args)) } else { None });

                    write_frame(
                        &mut stream,
                        OP_FRAME,
                        &serde_json::json!({ "cmd": "SET_ACTIVITY", "data": args["activity"], "evt": null, "nonce": frame["nonce"] }),
                    )
                    .await?;
                }
            }
            OP_PING => {
                write_frame(&mut stream, OP_PONG, &frame).await?;
            }
            OP_CLOSE => break,
            _ => {}
        }
    }

    on_update(None);
    Ok(())
}

/// Starts the presence server in the background. `on_update` is called
/// with `Some(..)` whenever a connected game reports activity, and `None`
/// when that game disconnects or clears its activity - callers decide how
/// to surface that (e.g. forward it to the webview as a Tauri event).
pub fn spawn<F>(on_update: F)
where
    F: Fn(Option<PresenceUpdate>) + Send + Sync + Clone + 'static,
{
    tauri::async_runtime::spawn(async move {
        let Some(listener) = bind_first_free().await else {
            tracing_log("no free discord-ipc-N slot (0-9 all taken), rich presence disabled");
            return;
        };

        loop {
            match listener.accept().await {
                Ok(stream) => {
                    let cb = on_update.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = handle_client(stream, move |update| cb(update)).await;
                    });
                }
                Err(_) => continue,
            }
        }
    });
}
