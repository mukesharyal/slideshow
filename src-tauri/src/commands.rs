use local_ip_address::local_ip;
use crate::server;
use crate::utility;
use tokio::runtime;
use tauri::{Emitter, State};

use tauri::{AppHandle, Manager};

use std::path::PathBuf;


use local_ip_address::list_afinet_netifas;

// The function to tell us whether the system is connected to a network or not
#[tauri::command]
pub fn is_connected() -> bool {
    match list_afinet_netifas() {
        Ok(network_interfaces) => {
            // Check if there's at least one non-loopback interface
            network_interfaces.iter().any(|(_name, ip)| !ip.is_loopback())
        }
        Err(_) => false
    }
}




// The function to get our WebSocket and HTTPS server up and running
#[tauri::command]
pub fn start_server(
    app_handle: tauri::AppHandle,
    current_state: tauri::State<'_, utility::CurrentSlideState>
) {

    let extracted_state = current_state.inner().clone();

    // Spawn a new worker thread for the server
    tauri::async_runtime::spawn(async move {
        // Since setup_server is likely async, we just await it
        server::setup_server(app_handle, extracted_state).await;
    });

}



#[tauri::command]
pub async fn open_slide_viewer(app: tauri::AppHandle, server_address: String, slide_number: u32) {

    let url_string = format!("slide-viewer?slideNumber={}&serverAddress={}", slide_number, server_address);

    let url = tauri::WebviewUrl::App(url_string.into());

    let parent_window = match app.get_webview_window("main") {

        Some(w) => w,

        None => {
            println!("Could not get the parent window!");
            return;
        }
    };

    tauri::WebviewWindowBuilder::new(&app, "slide_viewer", url)
        .parent(&parent_window)
        .unwrap()
        .title("Slide Viewer")
        .inner_size(800.0, 600.0)
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string());  
}



#[tauri::command]
pub async fn show_qr_code(app: tauri::AppHandle, server_address: String) {

    println!("Show QR Code function called!");

    let url_string = format!("qr-code?serverAddress={}", server_address);

    let url = tauri::WebviewUrl::App(url_string.into());

    let parent_window = match app.get_webview_window("main") {

        Some(w) => w,

        None => {
            println!("Could not get the parent window!");
            return;
        }
    };

    tauri::WebviewWindowBuilder::new(&app, "qr_code", url)
        .parent(&parent_window)
        .unwrap()
        .title("QR Code")
        .inner_size(400.0, 400.0)
        .resizable(false)
        .fullscreen(false)
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string());  
}




#[tauri::command]
pub async fn delete_slide(
    app: tauri::AppHandle, 
    slide_number: u16, 
    current_state: tauri::State<'_, utility::CurrentSlideState>
) -> Result<String, String> {
    
    let file_path = PathBuf::from("assets").join(format!("slide{}.png", slide_number));

    // Use tokio::fs instead of std::fs for async-friendly deletion
    match tokio::fs::remove_file(file_path).await {

        Ok(_) => {
            app.emit("slide_removed", slide_number).unwrap();

            // Scope the lock so it's released immediately after getting the list
            let deleted_list = {

                let mut lock = current_state.deleted_slides.lock().await;
                lock.push(slide_number);
                lock.clone() 
            }; 

            let message_construct = utility::ServerMessage {

                message_type: "slideDeleted".to_string(),
                current_state: utility::CurrentState {
                    num_slides: None,
                    deleted_slides: Some(deleted_list)
                },
            };

            let json_string = serde_json::to_string(&message_construct).unwrap();

            match current_state.broadcast_tx.send(json_string) {

                Ok(num) => Ok(format!("Notified {} clients", num)),
                Err(_) => Err("Broadcast failed".into()),
            }
        }
        Err(_) => Err("Could not delete the slide".into()),
    }
}