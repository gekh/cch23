use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    response::Response,
    routing::{get, post},
    Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use log::info;
use reqwest::StatusCode;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;

pub fn get_routes() -> Router {
    Router::new()
        .route("/19/ws/ping", get(ws_ping))
        .route("/19/reset", post(tweeter_reset))
        .route("/19/views", get(tweeter_views))
        .route(
            "/19/ws/room/:room_number/user/:username",
            get(tweeter_ws_handler),
        )
        .with_state(Arc::new(TweeterState {
            views: Arc::new(Mutex::new(0)),
            user_set: Mutex::new(HashSet::new()),
            rooms: Mutex::new(HashMap::new()),
        }))
}

async fn ws_ping(ws: WebSocketUpgrade) -> Response {
    info!("19 ws ping started");
    ws.on_upgrade(|socket| ws_ping_socket(socket))
}

async fn ws_ping_socket(mut socket: WebSocket) {
    let mut game_started = false;
    while let Some(msg) = socket.recv().await {
        let msg = if let Ok(msg) = msg {
            msg
        } else {
            // client disconnected
            return;
        };

        match msg {
            axum::extract::ws::Message::Text(text) => {
                if text == "ping" && game_started {
                    if let Err(err) = socket.send(Message::Text("pong".to_string())).await {
                        print!("Error: {:?}", err);
                    }
                } else if text == "serve" {
                    game_started = true;
                }
            }
            _ => {}
        }
    }
}

struct TweeterState {
    views: Arc<Mutex<u32>>,
    // We require unique usernames. This tracks which usernames have been taken.
    user_set: Mutex<HashSet<String>>,
    // Channels used to send messages to all connected clients.
    rooms: Mutex<HashMap<usize, broadcast::Sender<String>>>,
}

#[derive(serde::Deserialize, Debug)]
struct UserMsgIn {
    message: String,
}

#[derive(serde::Serialize, Debug)]
struct UserMsgOut {
    user: String,
    message: String,
}

async fn tweeter_reset(
    State(state): State<Arc<TweeterState>>,
) -> Result<String, (StatusCode, String)> {
    info!("19 tweeter reset started");
    let mut views = state.views.lock().expect("mutex was poisoned");
    *views = 0;

    Ok("OK".to_string())
}

async fn tweeter_views(
    State(state): State<Arc<TweeterState>>,
) -> Result<String, (StatusCode, String)> {
    info!("19 tweeter views started");
    let views = state.views.lock().expect("mutex was poisoned");
    info!("views: {views}");

    Ok(views.to_string())
}

async fn tweeter_ws_handler(
    Path((room_number, username)): Path<(usize, String)>,
    ws: WebSocketUpgrade,
    State(state): State<Arc<TweeterState>>,
) -> Response {
    info!("19 tweeter ws started");
    ws.on_upgrade(move |socket| tweeter_ws(socket, state, username, room_number))
}

async fn tweeter_ws(
    stream: WebSocket,
    state: Arc<TweeterState>,
    username: String,
    room_number: usize,
) {
    let (mut sender, mut receiver) = stream.split();

    if !check_username(&state, &username) {
        info!("Username {username} is already taken.");
        // Only send our client that username is taken.
        let _ = sender
            .send(Message::Text(String::from("Username already taken.")))
            .await;

        return;
    }

    info!("User {username} joined room {room_number}.");

    let room_tx = find_room(&state, &room_number);
    // We subscribe *before* sending the "joined" message, so that we will also
    // display it to our client.
    let mut rx = room_tx.subscribe();
    let views = state.views.clone();

    // Spawn the first task that will receive broadcast messages and send text
    // messages over the websocket to our client.
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            *views.lock().unwrap() += 1;
            // In any websocket error, break loop.
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Clone things we want to pass (move) to the receiving task.
    let tx = room_tx.clone();
    let name = username.clone();

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            let user_msg: UserMsgIn = serde_json::from_str(text.as_str()).unwrap();
            if user_msg.message.len() > 128 {
                continue;
            }

            let out = serde_json::json!(UserMsgOut {
                user: name.clone(),
                message: user_msg.message.clone(),
            })
            .to_string();
            let _ = tx.send(out.clone());
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    info!("User {username} left.");

    state.user_set.lock().unwrap().remove(&username);
}

fn check_username(state: &TweeterState, name: &str) -> bool {
    let mut user_set = state.user_set.lock().unwrap();

    if !user_set.contains(name) {
        user_set.insert(name.to_owned());

        return true;
    }

    false
}

fn find_room(state: &TweeterState, room_number: &usize) -> broadcast::Sender<String> {
    let mut rooms = state.rooms.lock().unwrap();

    if let Some(room_tx) = rooms.get(&room_number) {
        room_tx.clone()
    } else {
        let (tx, _) = broadcast::channel(100_000);
        rooms.insert(room_number.clone(), tx.clone());
        tx
    }
}
