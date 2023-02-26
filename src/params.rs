use rocket::{request::{FromRequest, Outcome}, async_trait, Request};

pub struct Params(pub String);

#[async_trait]
impl<'r> FromRequest<'r> for Params {
    type Error = String;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let params = request.headers()
            .iter()
            .filter(|header| header.name.starts_with("p-"))
            .map(|header| {
                format!("{}|{}\n", header.name[2..].as_str(), header.value)
            })
            .reduce(|a, b| a + &b)
            .unwrap_or_default()
            .to_lowercase();
            
        Outcome::Success(Params(params))
    }
}

pub struct Authenticated(pub bool);

#[async_trait]
impl<'r> FromRequest<'r> for Authenticated {
    type Error = std::convert::Infallible;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let headers = request.headers().get("Authorization");
        for header in headers {
            if crate::config::APIKEYS.contains(&header) {
                return Outcome::Success(Authenticated(true))
            }
        }
        Outcome::Success(Authenticated(false))
    }
}

