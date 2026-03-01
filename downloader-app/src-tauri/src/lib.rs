// Tauri desktop app for Downloader.
// Commands wire the Svelte frontend to `downloader_core` via Tauri IPC.

mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::start_download,
            commands::start_download_with_progress,
            commands::cancel_download,
            commands::list_projects,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
