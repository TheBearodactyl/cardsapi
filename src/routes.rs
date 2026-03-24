use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;

use crate::auth::PlayerId;
use crate::errors::ApiError;
use crate::games::GameState;
use crate::models::*;
use crate::state::*;



#[rocket::post("/api/rooms", data = "<body>")]
pub async fn create_room(
    player: PlayerId,
    body: Json<CreateRoomRequest>,
    state: &State<SharedState>,
) -> Result<(Status, Json<Room>), ApiError> {
    let player_info = state.ensure_player(player.0).await;
    let mut rooms = state.rooms.write().await;
    let code = generate_room_code(&rooms);

    let room = Room {
        code: code.clone(),
        game_type: body.game_type,
        host_id: player.0,
        players: vec![Player {
            id: player.0,
            name: player_info.name,
            connected: true,
        }],
        status: RoomStatus::Waiting,
        max_players: body.game_type.max_players(),
        min_players: body.game_type.min_players(),
        created_at: chrono::Utc::now(),
    };

    let room_state = RoomState {
        room: room.clone(),
        game: None,
        senders: std::collections::HashMap::new(),
    };

    rooms.insert(code, room_state);
    Ok((Status::Created, Json(room)))
}

#[rocket::get("/api/rooms/<code>")]
pub async fn get_room(
    _player: PlayerId,
    code: &str,
    state: &State<SharedState>,
) -> Result<Json<Room>, ApiError> {
    let rooms = state.rooms.read().await;
    let room_state = rooms.get(code).ok_or(ApiError::RoomNotFound)?;
    Ok(Json(room_state.room.clone()))
}

#[rocket::post("/api/rooms/<code>/join")]
pub async fn join_room(
    player: PlayerId,
    code: &str,
    state: &State<SharedState>,
) -> Result<Json<Room>, ApiError> {
    let player_info = state.ensure_player(player.0).await;
    let players_store = state.players.read().await;
    let mut rooms = state.rooms.write().await;
    let room_state = rooms.get_mut(code).ok_or(ApiError::RoomNotFound)?;

    if room_state.room.status != RoomStatus::Waiting {
        return Err(ApiError::GameAlreadyStarted);
    }

    
    if room_state.room.players.iter().any(|p| p.id == player.0) {
        return Ok(Json(room_state.room.clone()));
    }

    if room_state.room.players.len() as u32 >= room_state.room.max_players {
        return Err(ApiError::RoomFull);
    }

    let new_player = Player {
        id: player.0,
        name: player_info.name.clone(),
        connected: true,
    };

    room_state.room.players.push(new_player.clone());

    
    let envelope = WsEnvelope::new(
        "PLAYER_JOINED",
        serde_json::json!({
            "player": new_player,
            "playerCount": room_state.room.players.len()
        }),
    );
    room_state.broadcast(&envelope);

    drop(players_store);
    Ok(Json(room_state.room.clone()))
}

#[rocket::post("/api/rooms/<code>/leave")]
pub async fn leave_room(
    player: PlayerId,
    code: &str,
    state: &State<SharedState>,
) -> Result<Json<MessageResponse>, ApiError> {
    let mut rooms = state.rooms.write().await;
    let room_state = rooms.get_mut(code).ok_or(ApiError::RoomNotFound)?;

    let player_index = room_state
        .room
        .players
        .iter()
        .position(|p| p.id == player.0)
        .ok_or(ApiError::PlayerNotInRoom)?;

    let player_name = room_state.room.players[player_index].name.clone();
    let was_host = room_state.room.host_id == player.0;

    room_state.room.players.remove(player_index);
    room_state.senders.remove(&player.0);

    if room_state.room.players.is_empty() {
        
        let envelope = WsEnvelope::new(
            "ROOM_CLOSED",
            serde_json::json!({
                "reason": "All players have left the room"
            }),
        );
        room_state.broadcast(&envelope);
        rooms.remove(code);
        return Ok(Json(MessageResponse {
            message: "Successfully left the room".into(),
        }));
    }

    
    let new_host_id = if was_host {
        let new_host = room_state.room.players[0].id;
        room_state.room.host_id = new_host;
        Some(new_host)
    } else {
        None
    };

    
    let envelope = WsEnvelope::new(
        "PLAYER_LEFT",
        serde_json::json!({
            "playerId": player.0,
            "playerName": player_name,
            "newHostId": new_host_id,
            "playerCount": room_state.room.players.len()
        }),
    );
    room_state.broadcast(&envelope);

    Ok(Json(MessageResponse {
        message: "Successfully left the room".into(),
    }))
}

#[rocket::post("/api/rooms/<code>/start")]
pub async fn start_game(
    player: PlayerId,
    code: &str,
    state: &State<SharedState>,
) -> Result<Json<Room>, ApiError> {
    let players_store = state.players.read().await;
    let mut rooms = state.rooms.write().await;
    let room_state = rooms.get_mut(code).ok_or(ApiError::RoomNotFound)?;

    if room_state.room.host_id != player.0 {
        return Err(ApiError::NotHost);
    }

    if room_state.room.status != RoomStatus::Waiting {
        return Err(ApiError::GameAlreadyStarted);
    }

    if (room_state.room.players.len() as u32) < room_state.room.min_players {
        return Err(ApiError::InvalidAction(
            "Not enough players to start".into(),
        ));
    }

    room_state.room.status = RoomStatus::InProgress;

    let player_ids: Vec<uuid::Uuid> = room_state.room.players.iter().map(|p| p.id).collect();
    let game = GameState::init(room_state.room.game_type, &player_ids);

    let player_order: Vec<String> = game.player_order().iter().map(|id| id.to_string()).collect();

    room_state.game = Some(game);

    
    let envelope = WsEnvelope::new(
        "GAME_STARTED",
        serde_json::json!({
            "gameType": room_state.room.game_type,
            "playerOrder": player_order
        }),
    );
    room_state.broadcast(&envelope);

    
    room_state.broadcast_game_state(&players_store);

    drop(players_store);
    Ok(Json(room_state.room.clone()))
}



#[rocket::get("/api/rooms/<code>/state")]
pub async fn get_game_state(
    player: PlayerId,
    code: &str,
    state: &State<SharedState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let players_store = state.players.read().await;
    let rooms = state.rooms.read().await;
    let room_state = rooms.get(code).ok_or(ApiError::RoomNotFound)?;

    if !room_state.room.players.iter().any(|p| p.id == player.0) {
        return Err(ApiError::PlayerNotInRoom);
    }

    let game = room_state
        .game
        .as_ref()
        .ok_or(ApiError::InvalidAction("Game has not started".into()))?;

    let names = room_state.player_names(&players_store);
    let view = game.get_state_for_player(player.0, &names);

    Ok(Json(view))
}

#[rocket::post("/api/rooms/<code>/action", data = "<body>")]
pub async fn submit_action(
    player: PlayerId,
    code: &str,
    body: Json<GameAction>,
    state: &State<SharedState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let players_store = state.players.read().await;
    let mut rooms = state.rooms.write().await;
    let room_state = rooms.get_mut(code).ok_or(ApiError::RoomNotFound)?;

    if !room_state.room.players.iter().any(|p| p.id == player.0) {
        return Err(ApiError::PlayerNotInRoom);
    }

    let names = room_state.player_names(&players_store);
    let player_name = names
        .get(&player.0)
        .cloned()
        .unwrap_or_default();

    let game = room_state
        .game
        .as_mut()
        .ok_or(ApiError::InvalidAction("Game has not started".into()))?;

    let result = game.process_action(player.0, body.into_inner(), &names)?;

    
    let action_envelope = WsEnvelope::new(
        "GAME_ACTION_RESULT",
        serde_json::json!({
            "playerId": player.0,
            "playerName": player_name,
            "action": result.action_name,
            "description": result.description,
            "result": result.result_label,
            "details": result.details
        }),
    );
    room_state.broadcast(&action_envelope);

    
    room_state.broadcast_game_state(&players_store);

    
    if let Some(game_over) = &result.game_over {
        room_state.room.status = RoomStatus::Finished;

        let game_over_envelope = WsEnvelope::new(
            "GAME_OVER",
            serde_json::json!({
                "winnerId": game_over.winner_id,
                "winnerName": game_over.winner_name,
                "finalScores": game_over.final_scores,
                "reason": game_over.reason
            }),
        );
        room_state.broadcast(&game_over_envelope);
    }

    
    let game = room_state.game.as_ref().unwrap();
    let view = game.get_state_for_player(player.0, &names);

    drop(players_store);
    Ok(Json(view))
}



#[rocket::get("/api/players/me")]
pub async fn get_player(
    player: PlayerId,
    state: &State<SharedState>,
) -> Json<PlayerResponse> {
    let info = state.ensure_player(player.0).await;
    Json(PlayerResponse {
        id: info.id,
        name: info.name,
        created_at: info.created_at,
    })
}

#[rocket::put("/api/players/me", data = "<body>")]
pub async fn update_player(
    player: PlayerId,
    body: Json<UpdateNameRequest>,
    state: &State<SharedState>,
) -> Result<Json<PlayerResponse>, ApiError> {
    let name = body.into_inner().name;
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.len() > 32 {
        return Err(ApiError::InvalidPayload(
            "Name must be 1-32 characters".into(),
        ));
    }

    state.ensure_player(player.0).await;

    let mut players = state.players.write().await;
    let info = players.get_mut(&player.0).unwrap();
    info.name = trimmed.to_string();

    Ok(Json(PlayerResponse {
        id: info.id,
        name: info.name.clone(),
        created_at: info.created_at,
    }))
}
