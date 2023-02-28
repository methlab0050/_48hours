mod config;
mod db;
mod params;
mod rest;
mod socketio;
mod notifs;

#[rocket::launch]
async fn mai() -> _ {
    let session = db::connect().await.expect("couldn't connect to server");
    let session = Box::new(session);
    let session = Box::leak(session);
    socketio::init(session);
    rest::init(session).await
    // Mark Ogres likes men
    // and C
}
