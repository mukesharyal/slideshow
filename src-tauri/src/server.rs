use std::net::{SocketAddr, IpAddr};
use local_ip_address::local_ip;

use axum::{
    extract::ws::{WebSocketUpgrade, Message, WebSocket},
    response::{Response, Html},
    routing::get,
    Router,
    serve
};

use tokio::sync::mpsc;

use local_ip_address::list_afinet_netifas;

use futures_util::stream::StreamExt;
use futures_util::sink::SinkExt;
use futures_util::stream::SplitSink;

use xcap;

use axum::extract::State;

use tauri::{AppHandle, Emitter};
use tokio::{net::TcpListener, try_join};

use tokio::task; 
    use std::fs; 

use crate::utility;

use std::path::{Path, PathBuf};


use axum::{
    http::header::{CONTENT_TYPE, HeaderValue},
    response::IntoResponse,
};

use tower_http::services::ServeDir;

use rdev::{listen, Event, EventType};

use rdev::EventType::KeyRelease;
use rdev::Key::{RightArrow, KeyJ, LeftArrow};

use tokio::sync::{Mutex, broadcast};
use std::sync::Arc;

type SharedWebSocketSender = Arc<Mutex<SplitSink<WebSocket, Message>>>;


// This function handles the synchronization, capture, and save.
async fn take_screenshot_and_save(app_handle: AppHandle, current_state: utility::CurrentSlideState) {

    println!("The screenshot function is called!");
    

    let slide_number = {

        let mut counter = current_state.slide_number.lock().await;

        *counter += 1; // Increment the counter

        current_state.change_volatile_slide_number(&app_handle, true).await; // Increment the volatile slide counter as well

        let number = *counter;
        number
        
    };
    
    let file_name = format!("slide{}.png", slide_number);
    let output_path = PathBuf::from("assets").join(&file_name);
    

    let monitors = xcap::Monitor::all().unwrap();

    if let Some(first_monitor) = monitors.into_iter().next() {
        
        // 3. Capture the image from the first monitor
        let image = first_monitor.capture_image().unwrap();

        // 4. Save the image
        image.save(&output_path).unwrap();

        println!("Screenshot saved.");

        // Send the event to the app frontend as well
        app_handle.emit("new_slide", slide_number);

        let message_construct = utility::ServerMessage {
            message_type: "slideAdded".to_string(),
            current_state: utility::CurrentState{
                num_slides: Some(slide_number),
                deleted_slides: None
            },
        };

        // 1. Convert struct to JSON String
        let json_string = serde_json::to_string(&message_construct).unwrap();


        match current_state.broadcast_tx.send(json_string) {
        Ok(num_receivers) => {
            println!("Successfully broadcasted message to {} WebSocket clients.", num_receivers);
        }
        Err(e) => {
            eprintln!("Failed to broadcast message: {:?}", e);
        }
    }

    } else {
        println!("❌ No monitors were detected to capture.");
    }

}

async fn handle_key_press(event: Event, current_state: utility::CurrentSlideState, app_handle: AppHandle) {

    let current_state_clone = current_state.clone();

    match event.event_type {

        KeyRelease(KeyJ) => {

            println!("Key J pressed!");

            tauri::async_runtime::spawn(async move {

                take_screenshot_and_save(app_handle.clone(), current_state_clone).await;
            });
        }

        KeyRelease(RightArrow) => {

            // Lock the mutex and free it immediately to prevent Mutex deadlocks
            let slide_number = {

                let lock = current_state.slide_number.lock().await;
                *lock
            };

            let volatile_slide_number = {

                let lock = current_state.volatile_slide_number.lock().await;
                *lock
            };

            if volatile_slide_number == slide_number {

                tauri::async_runtime::spawn(async move {

                    // We are now in the next slide, so we wait for a second for the animations (or loading images)
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                    take_screenshot_and_save(app_handle.clone(), current_state_clone).await;
                });

            } else {

                current_state.change_volatile_slide_number(&app_handle, true).await;
            }
        }

        KeyRelease(LeftArrow) => {

            current_state.change_volatile_slide_number(&app_handle, false).await;
        }

        _ => {}
    }
}

fn http_app() -> Router {

    // Create the service to serve files from the "assets" directory
    let static_files_service = ServeDir::new("assets");

    // Create the router
    Router::new()
        // If the request doesn't match any specific route (like /), 
        // it falls back to checking the static files directory.
        // Request to /address.js will look for assets/address.js.
        .fallback_service(static_files_service.clone())
        .route("/", get(provide_app))
}

fn ws_app(current_state:utility::CurrentSlideState) -> Router {

    Router::new()
        .route("/", get(ws_handler))
        .with_state(current_state)
}

// The main WebSocket handler function
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(current_state): State<utility::CurrentSlideState>
) -> Response {

    ws.on_upgrade(|socket: WebSocket| async move {

        // 1. Split the socket into a sink (sender) and a stream (receiver).
        let (sender, mut receiver) = socket.split();

        // 2. Wrap the sender in a shared Arc/Mutex for concurrent access.
        let shared_sender: SharedWebSocketSender = Arc::new(Mutex::new(sender));
        
        // --- Clone the shared sender for the broadcast task ---
        let broadcast_sender_clone = shared_sender.clone();
        
        // --- 🎯 FIX: SEND INITIAL SLIDE NUMBER IMMEDIATELY UPON CONNECT ---
        {

            // Lock the slide state to get the current number
            let slide_number = *current_state.slide_number.lock().await; 

            let deleted_slides_number = current_state.deleted_slides.lock().await.clone();


            let message_construct = utility::ServerMessage {
                message_type: "slidesInfo".to_string(),
                current_state: utility::CurrentState{
                    num_slides: Some(slide_number),
                    deleted_slides: Some(deleted_slides_number)
                }

            };

            // 1. Convert struct to JSON String
            let json_string = serde_json::to_string(&message_construct).unwrap();

            // 2. Wrap it in a WebSocket Text Message
            let sent_message = Message::Text(json_string.into());

            // Lock the sender to send the initial message
            let mut sink_lock = shared_sender.lock().await;

            if sink_lock.send(sent_message).await.is_err() {
                // If the initial send fails, the connection is likely dropped.
                eprintln!("Failed to send initial slide number; client disconnected.");
                return; // Exit the upgrade closure early
            }
        } // sink_lock is dropped, releasing the sender Mutex.

        // Get the broadcast receiver for fanout
        let mut rx = current_state.broadcast_tx.subscribe();
        
        // 3. CONCURRENT TASK (Outgoing: Broadcast Listener)
        tauri::async_runtime::spawn(async move {
            let sender = broadcast_sender_clone; 

            loop {
                match rx.recv().await {
                    Ok(msg) => {
                        let ws_message = Message::Text(msg.into());
                        let mut sink_lock = sender.lock().await;
                        if sink_lock.send(ws_message).await.is_err() {
                            break; 
                        }
                    }
                    // Handle lagging separately so the client doesn't get kicked 
                    // just because they were a millisecond slow once
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        eprintln!("Client lagged behind by {} messages", skipped);
                        continue; // Keep the loop alive
                    }
                    Err(_) => break, // Channel closed
                }
            }
        });

        // 4. MAIN LOOP (Incoming: Client Listener - Simplified)
        // We only listen for close messages or errors to gracefully clean up.
        // All "join" logic is removed.
        while let Some(msg_result) = receiver.next().await { 
            match msg_result {
                Ok(Message::Close(_)) | Err(_) => {
                    // Connection closed by the client or stream error occurred.
                    println!("WebSocket client closed connection.");
                    break;
                }
                // Ignore all other incoming messages (Text, Binary, Ping/Pong)
                _ => {}
            }
        }
    })
}


pub async fn setup_server(app_handle: AppHandle, current_state: utility::CurrentSlideState) {

    let extracted_current_state = current_state.clone();

    let current_state_clone = extracted_current_state.clone();

    // Get the host ip address of the system
    let host_ip: IpAddr = match list_afinet_netifas() {
        Ok(network_interfaces) => {
            network_interfaces.iter()
                .find(|(_name, ip)| !ip.is_loopback())
                .map(|(_name, ip)| *ip)
                .unwrap()
        }
        Err(_) => {
            println!("Something went wrong!");
            return;
        }
    };


    // Set the port address to 0 to let the OS decide the port number for us
    let port: u16 = 0;


    // Create the address for our server
    let http_address = SocketAddr::new(host_ip, port);
    let ws_address = SocketAddr::new(host_ip, port);


    let http_listener = match TcpListener::bind(http_address).await {

        Ok(l) => l,

        Err(_) => {

            app_handle.emit("server_start_failure", ());

            println!("Could not create the http server!");
            return;
        }
    };

    let http_addr_str = http_listener.local_addr().unwrap().to_string();

    println!("The http server was started at {}", http_addr_str);
    

    let http_server = async move {


        serve(http_listener, http_app().into_make_service())
            .await
            .map_err(|e| format!("HTTP Server Error: {}", e))
    };



    let ws_listener = match TcpListener::bind(ws_address).await {

        Ok(l) => l,

        Err(_) => {

            app_handle.emit("server_start_failure", ());

            println!("Could not create the ws server!");
            return;
        }
    };

    let ws_addr_str = ws_listener.local_addr().unwrap().to_string();


    let file_content = format!("export const webSocketAddress = \"ws://{}\";\n", ws_addr_str);
    let file_path = PathBuf::from("assets").join("address.js");
    let file_dir = PathBuf::from("assets");


    let write_result = task::spawn_blocking(move || {

        match fs::remove_dir_all(&file_dir) {
            Ok(_) => {
                println!("✅ Successfully removed old directory: {:?}", &file_dir);
            },
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
                // This is the expected and safe case if the directory is missing
                println!("ℹ️ Directory {:?} not found, proceeding to create.", &file_dir);
            },
            Err(e) => {
                // Fail if the directory exists but we can't remove it (e.g., permissions)
                return Err(e);
            }
        }


        fs::create_dir_all(&file_dir)?;
        fs::write(&file_path, file_content)?;
        Ok::<(), std::io::Error>(())


    }).await;

    match write_result {

        Ok(Ok(_)) => println!("✅ address.js written successfully."),

        _ => {
            // Log the error and consider this a fatal setup failure
            println!("Fatal: Failed to write address.js file.");
            app_handle.emit("server_start_failure", ());
            return;
        }
    }
    

    let ws_server = async move {

        serve(ws_listener, ws_app(extracted_current_state.clone()).into_make_service())
            .await
            .map_err(|e| format!("HTTP Server Error: {}", e))
    };

    app_handle.emit("server_ready", http_addr_str);

    println!("The ws server was started at {}", ws_addr_str);

    let app_handle_clone = app_handle.clone();

    let (tx, mut rx) = mpsc::unbounded_channel();

    // 2. Start the Async "Worker"
    tokio::spawn(async move {
        while let Some((event, state, app)) = rx.recv().await {
            // This is where your async function is finally called and awaited
            handle_key_press(event, state, app).await;
        }
    });

    // 3. The Listener Thread
    std::thread::spawn(move || {
        let callback = move |event: Event| {
            // We just send the data and keep moving. 
            // This takes almost zero time, so the keyboard doesn't lag.
            let _ = tx.send((event, current_state_clone.clone(), app_handle_clone.clone()));
        };

        if let Err(error) = listen(callback) {
            println!("Error: {:?}", error)
        }
    });

    if let Err(_) = try_join!(tokio::spawn(http_server), tokio::spawn(ws_server))
    {

        println!(" A server crashed unexpectedly.");
        app_handle.emit("server_crash", ());

    }
    else
    {
        println!("Both servers shut down gracefully.");
    }
    
}



async fn provide_app() -> Html<&'static str> {
    // This loads the file content INTO the binary during compilation
    const HTML: &str = include_str!("../index.html");
    
    Html(HTML)
}