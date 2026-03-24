use rocket::http::Status;
use rocket::request::{self, FromRequest, Request};
use uuid::Uuid;

pub struct PlayerId(pub Uuid);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for PlayerId {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        match req.headers().get_one("X-Player-ID") {
            Some(id) => match Uuid::parse_str(id) {
                Ok(uuid) => request::Outcome::Success(PlayerId(uuid)),
                Err(_) => request::Outcome::Error((Status::Unauthorized, ())),
            },
            None => request::Outcome::Error((Status::Unauthorized, ())),
        }
    }
}
