// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[cfg(target_os = "linux")]
mod audio_output;
mod media_bridge;
mod rpc;

use tauri::Emitter;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init());

    #[cfg(target_os = "linux")]
    let builder = builder.invoke_handler(tauri::generate_handler![
        greet,
        audio_output::list_audio_sinks,
        audio_output::get_default_audio_sink,
        audio_output::set_app_audio_sink
    ]);
    #[cfg(not(target_os = "linux"))]
    let builder = builder.invoke_handler(tauri::generate_handler![greet]);

    builder
        .setup(|app| {
            let handle = app.handle().clone();
            rpc::spawn(move |update| {
                let _ = handle.emit("rich-presence", update);
            });

            let handle = app.handle().clone();
            media_bridge::spawn(move |update| {
                let _ = handle.emit("media-presence", update);
            });

            #[cfg(target_os = "linux")]
            {
                use tauri::Manager;

                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.with_webview(|webview| {
                        use webkit2gtk::glib::prelude::*;
                        use webkit2gtk::{PermissionRequestExt, SettingsExt, WebViewExt};

                        let wv = webview.inner();

                        if let Some(settings) = wv.settings() {
                            settings.set_enable_media_stream(true);
                            settings.set_enable_webrtc(true);
                            settings.set_enable_mediasource(true);
                        }

                        wv.connect_permission_request(|_, request| {
                            if request.is::<webkit2gtk::UserMediaPermissionRequest>() {
                                request.allow();
                                true
                            } else {
                                false
                            }
                        });
                    });
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
