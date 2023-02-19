mod config;
mod db;
use rocket::{get, post, request::{FromRequest, Request, Outcome}, async_trait, response::Response};
use serde_json::json;

struct Authenticated(bool);

#[async_trait]
impl<'r> FromRequest<'r> for Authenticated {
    type Error = std::convert::Infallible;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let headers = request.headers().get("Authorization");
        for header in headers {
            if config::APIKEYS.contains(&header) {
                return Outcome::Success(Authenticated(true))
            }
        }
        Outcome::Success(Authenticated(false))
    }
}

#[get("/fetch")]
async fn fetch_email(auth: Authenticated) {
    if auth.0 {
        
    }

    // let data = db::fetch_email().await;
}
// #[post("/add")]
// #[post("/invalidate")]
// #[get("/")]

fn main() {
    println!("Hello, world!");
}
