use rocket::futures::{SinkExt, StreamExt};
use rocket::State;
use rocket_ws as ws;
use uuid::Uuid;

use crate::models::WsEnvelope;
use crate::state::SharedState;

#[rocket::get("/ws?<roomCode>&<playerId>")]
#[allow(non_snake_case)]
pub async fn websocket(
    ws: ws::WebSocket,
    roomCode: &str,
    playerId: &str,
    state: &State<SharedState>,
) -> Result<ws::Channel<'static>, rocket::http::Status> {
    let player_id = Uuid::parse_str(playerId).map_err(|_| rocket::http::Status::Unauthorized)?;

    let room_code = roomCode.to_string();

    
    {
        let rooms = state.rooms.read().await;
        let room_state = rooms.get(&room_code).ok_or(rocket::http::Status::NotFound)?;
        if !room_state.room.players.iter().any(|p| p.id == player_id) {
            return Err(rocket::http::Status::Forbidden);
        }
    }

    let state = state.inner().clone();
    let room_code_clone = room_code.clone();

    Ok(ws.channel(move |stream| {
        Box::pin(async move {
            let (mut sink, mut source) = stream.split();
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

            
            {
                let mut rooms = state.rooms.write().await;
                if let Some(room_state) = rooms.get_mut(&room_code_clone) {
                    room_state.senders.insert(player_id, tx.clone());
                    if let Some(p) = room_state
                        .room
                        .players
                        .iter_mut()
                        .find(|p| p.id == player_id)
                    {
                        p.connected = true;
                    }
                }
            }

            
            {
                let rooms = state.rooms.read().await;
                let players_store = state.players.read().await;
                if let Some(room_state) = rooms.get(&room_code_clone) {
                    if let Some(ref game) = room_state.game {
                        let names = room_state.player_names(&players_store);
                        let game_state = game.get_state_for_player(player_id, &names);
                        let envelope = WsEnvelope::new("GAME_STATE_UPDATE", game_state);
                        let msg = serde_json::to_string(&envelope).unwrap();
                        let _ = tx.send(msg);
                    }
                }
            }

            
            let send_task = tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    if sink.send(ws::Message::Text(msg)).await.is_err() {
                        break;
                    }
                }
            });

            
            while let Some(msg) = source.next().await {
                if msg.is_err() {
                    break;
                }
            }

            
            send_task.abort();
            {
                let mut rooms = state.rooms.write().await;
                if let Some(room_state) = rooms.get_mut(&room_code_clone) {
                    room_state.senders.remove(&player_id);
                    if let Some(p) = room_state
                        .room
                        .players
                        .iter_mut()
                        .find(|p| p.id == player_id)
                    {
                        p.connected = false;
                    }
                }
            }

            Ok(())
        })
    }))
}
