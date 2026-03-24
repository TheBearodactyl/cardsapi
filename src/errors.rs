use rocket::http::{ContentType, Status};
use rocket::response::{self, Responder, Response};
use rocket::Request;
use serde::Serialize;
use std::io::Cursor;

#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Debug)]
pub enum ApiError {
    RoomNotFound,
    RoomFull,
    GameAlreadyStarted,
    NotYourTurn,
    InvalidAction(String),
    NotHost,
    PlayerNotInRoom,
    InvalidPlayerId,
    InvalidPayload(String),
    RoomClosed,
}

impl ApiError {
    fn status_and_detail(&self) -> (Status, &str, String) {
        match self {
            ApiError::RoomNotFound => (
                Status::NotFound,
                "ROOM_NOT_FOUND",
                "The specified room code does not exist".into(),
            ),
            ApiError::RoomFull => (
                Status::Conflict,
                "ROOM_FULL",
                "This room already has the maximum number of players".into(),
            ),
            ApiError::GameAlreadyStarted => (
                Status::Conflict,
                "GAME_ALREADY_STARTED",
                "The game is already in progress".into(),
            ),
            ApiError::NotYourTurn => (
                Status::Forbidden,
                "NOT_YOUR_TURN",
                "It is not your turn".into(),
            ),
            ApiError::InvalidAction(msg) => (Status::BadRequest, "INVALID_ACTION", msg.clone()),
            ApiError::NotHost => (
                Status::Forbidden,
                "NOT_HOST",
                "Only the room host can perform this action".into(),
            ),
            ApiError::PlayerNotInRoom => (
                Status::BadRequest,
                "PLAYER_NOT_IN_ROOM",
                "You are not a member of this room".into(),
            ),
            ApiError::InvalidPlayerId => (
                Status::Unauthorized,
                "INVALID_PLAYER_ID",
                "Missing or invalid X-Player-ID header".into(),
            ),
            ApiError::InvalidPayload(msg) => (Status::BadRequest, "INVALID_PAYLOAD", msg.clone()),
            ApiError::RoomClosed => (
                Status::Gone,
                "ROOM_CLOSED",
                "The room has been closed and is no longer accessible".into(),
            ),
        }
    }
}

impl<'r> Responder<'r, 'static> for ApiError {
    fn respond_to(self, _req: &'r Request<'_>) -> response::Result<'static> {
        let (status, code, message) = self.status_and_detail();
        let body = serde_json::to_string(&ErrorResponse {
            error: ErrorDetail {
                code: code.to_string(),
                message,
            },
        })
        .unwrap();

        Response::build()
            .status(status)
            .header(ContentType::JSON)
            .sized_body(body.len(), Cursor::new(body))
            .ok()
    }
}
