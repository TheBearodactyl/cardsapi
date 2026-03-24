mod auth;
mod errors;
mod games;
mod models;
mod routes;
mod state;
mod ws;

use rocket::http::ContentType;
use rocket::Request;
use std::sync::Arc;

#[rocket::catch(401)]
fn unauthorized(_req: &Request) -> (ContentType, &'static str) {
    (
        ContentType::JSON,
        r#"{"error":{"code":"INVALID_PLAYER_ID","message":"Missing or invalid X-Player-ID header"}}"#,
    )
}

#[rocket::catch(404)]
fn not_found(_req: &Request) -> (ContentType, &'static str) {
    (
        ContentType::JSON,
        r#"{"error":{"code":"ROOM_NOT_FOUND","message":"The requested resource was not found"}}"#,
    )
}

#[rocket::catch(422)]
fn unprocessable(_req: &Request) -> (ContentType, &'static str) {
    (
        ContentType::JSON,
        r#"{"error":{"code":"INVALID_PAYLOAD","message":"The request body is invalid"}}"#,
    )
}

#[rocket::launch]
fn rocket() -> _ {
    let state = Arc::new(state::AppState::new());

    rocket::build()
        .manage(state as state::SharedState)
        .mount(
            "/",
            rocket::routes![
                routes::create_room,
                routes::get_room,
                routes::join_room,
                routes::leave_room,
                routes::start_game,
                routes::submit_action,
                routes::get_player,
                routes::update_player,
                ws::websocket,
            ],
        )
        .register("/", rocket::catchers![unauthorized, not_found, unprocessable])
}
