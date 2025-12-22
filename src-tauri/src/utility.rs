use tokio::sync::{Mutex, broadcast};
use std::sync::Arc;
use serde::Serialize;
use tauri::Emitter;


pub struct AppState {
    pub slide_number: Mutex<u16>, // the current slide number (for naming the slides)
    pub broadcast_tx: broadcast::Sender<String>, // transmitter to send the slide addition message
    pub deleted_slides: Mutex<Vec<u16>>, // holds the deleted slides
    pub volatile_slide_number: Mutex<u16> // the slide at which the user is currently at (by using the arrow keys)
}


pub type CurrentSlideState = Arc<AppState>;


// Automatically sync the frontend with the volatile slide number so the user knows which slide they're at to sync it
impl AppState {
    pub async fn change_volatile_slide_number(&self, app: &tauri::AppHandle, increment: bool) {

        let mut num = self.volatile_slide_number.lock().await;

        // If the volatile slide number is 1, this means the user is aggressively pressing the back arrow
        if(!increment && *num == 1)
        {
            return;
        }

        if(increment)
        {
            *num += 1;
        }
        else
        {
            *num -= 1;
        }
        
        let new_value = *num;

        app.emit("volatile_slide_changed", new_value);
    }
}


// The defined signature for the data sent by the server to the clients
// Has three properties: [type of message, numSlides, deletedSlides]
#[derive(Serialize)]
pub struct ServerMessage {
    pub message_type: String,
    pub current_state: CurrentState
}

#[derive(Serialize)]
pub struct CurrentState {
    pub num_slides: Option<u16>,
    pub deleted_slides: Option<Vec<u16>>
}