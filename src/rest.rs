use std::{sync::atomic::AtomicUsize, path::PathBuf};

use rocket::{get, post, serde::json::{json, Value, Json}, http::Status, routes, Ignite, Rocket};

use crate::{params::{Authenticated, Params}, db::{Keyspace, init, Combo}};

fn failure(code: u16, message: &str) -> (Status, Value) {
    let status = unsafe { Status::from_code(code).unwrap_unchecked() };

    (status, json!({
        "success": false,
        "message": message
    }))
}

#[get("/fetch/<_path..>")]
fn fetch(auth: Authenticated, mut keyspace: Keyspace<'_>, _path: PathBuf) -> (Status, Value) {
    if !auth.0 {
        return failure(401, "Not authenticated");
    }

    keyspace.fetch_email()
}

#[post("/add/<_path..>", data = "<body>")]
fn add(auth: Authenticated, mut keyspace: Keyspace<'_>, _path: PathBuf, body: &str, params: Params) -> (Status, Value) {
    if !auth.0 {
        return failure(401, "Not authenticated");
    }

    let mut payload = Vec::new();

    for line in body.replace("\r", "").lines() {
        let line = line.trim();
        let Some(index) = line.find(":") else { continue; };
        let email = line[..=index].to_string();
        let password = line[index+1..].to_string();

        payload.push(Combo { email, password });
    }

    let len = payload.len();

    match keyspace.add_combos(payload, &params.0) {
        Some(errors) => {
            (Status::Ok, json!({
                "success": true,
                "message": format!("tried to add {} combos (see errors)", len),
                "errors": errors
            }))
        }
        None => {
            (Status::Ok, json!({
                "success": true,
                "message": format!("Successfully added {} combos", len)
            }))
        }
    }
}

#[post("/invalidate/<_path..>", data = "<body>")]
fn invalidate(auth: Authenticated, keyspace: Keyspace<'_>, _path: PathBuf, body: Json<Value>) -> (Status, Value) {
    if !auth.0 {
        return failure(401, "Not authenticated");
    }

    let uuid = body["uuid"].as_str().unwrap();
    match keyspace.invalidate_combo(uuid.to_owned()) {
        Some(err) => {
            (Status::InternalServerError, json!({
                "success": false,
                "message": "could not remove combo",
                "errors": err.to_string()
            }))
        },
        None => {
            (Status::Ok, json!({
                "sucess": true,
                "message": "Successfully invalidated combo"
            }))
        }
    }
}

#[rocket::main]
pub async fn rocket() -> Result<Rocket<Ignite>, rocket::Error> {
    let session = unsafe { init() };
    rocket::build()
        .manage(session)
        .mount("/", routes![fetch, add, invalidate])
        .launch()
        .await
}
