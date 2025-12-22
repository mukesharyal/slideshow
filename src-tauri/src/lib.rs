mod commands;
mod utility;
mod server;

use tokio::sync::{Mutex, broadcast};

use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {

    let (tx, rx) = broadcast::channel(100); // Buffer size of 100 messages

    drop(rx);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())

        .manage(Arc::new(utility::AppState {
            slide_number: Mutex::new(0),
            broadcast_tx: tx,
            deleted_slides: Mutex::new(Vec::new()),
            volatile_slide_number: Mutex::new(0)
        }))


        .invoke_handler(tauri::generate_handler![

            commands::is_connected,
            commands::start_server,
            commands::open_slide_viewer,
            commands::delete_slide,
            commands::show_qr_code

        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
