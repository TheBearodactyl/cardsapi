use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use crate::games::GameState;
use crate::models::*;

pub type SharedState = Arc<AppState>;

pub struct AppState {
    pub rooms: RwLock<HashMap<String, RoomState>>,
    pub players: RwLock<HashMap<Uuid, PlayerInfo>>,
    pub player_counter: AtomicU32,
}

pub struct PlayerInfo {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

pub struct RoomState {
    pub room: Room,
    pub game: Option<GameState>,
    pub senders: HashMap<Uuid, mpsc::UnboundedSender<String>>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            rooms: RwLock::new(HashMap::new()),
            players: RwLock::new(HashMap::new()),
            player_counter: AtomicU32::new(0),
        }
    }

    pub async fn ensure_player(&self, player_id: Uuid) -> PlayerInfo {
        let mut players = self.players.write().await;
        if let Some(info) = players.get(&player_id) {
            return PlayerInfo {
                id: info.id,
                name: info.name.clone(),
                created_at: info.created_at,
            };
        }
        let count = self.player_counter.fetch_add(1, Ordering::Relaxed) + 1;
        let info = PlayerInfo {
            id: player_id,
            name: format!("Player {}", count),
            created_at: Utc::now(),
        };
        let result = PlayerInfo {
            id: info.id,
            name: info.name.clone(),
            created_at: info.created_at,
        };
        players.insert(player_id, info);
        result
    }
}

impl RoomState {
    pub fn player_names(&self, players: &HashMap<Uuid, PlayerInfo>) -> HashMap<Uuid, String> {
        self.room
            .players
            .iter()
            .map(|p| {
                let name = players
                    .get(&p.id)
                    .map(|info| info.name.clone())
                    .unwrap_or_else(|| p.name.clone());
                (p.id, name)
            })
            .collect()
    }

    pub fn broadcast(&self, envelope: &WsEnvelope) {
        let msg = serde_json::to_string(envelope).unwrap();
        for sender in self.senders.values() {
            let _ = sender.send(msg.clone());
        }
    }

    pub fn broadcast_game_state(&self, players: &HashMap<Uuid, PlayerInfo>) {
        if let Some(ref game) = self.game {
            let names = self.player_names(players);
            for player in &self.room.players {
                if let Some(sender) = self.senders.get(&player.id) {
                    let state = game.get_state_for_player(player.id, &names);
                    let envelope = WsEnvelope::new("GAME_STATE_UPDATE", state);
                    let msg = serde_json::to_string(&envelope).unwrap();
                    let _ = sender.send(msg);
                }
            }
        }
    }
}

pub fn generate_room_code(existing: &HashMap<String, RoomState>) -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    loop {
        let code: String = (0..4)
            .map(|_| (b'A' + rng.random_range(0..26u8)) as char)
            .collect();
        if !existing.contains_key(&code) {
            return code;
        }
    }
}
