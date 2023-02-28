use std::error::Error;

use cassandra_cpp::Session;
use rocket::{serde::json::{Value, serde_json::json, self}};
use rust_socketio::{RawClient, Payload, ClientBuilder};

use crate::{db::Keyspace, notifs::notify, config::{APIKEYS, CONFIG}};

/*
@socketio.event
def invalid(uuid):
    db.invalidateCombo(uuid)
*/
/*
@socketio.event
def validate(data):
    comboId = data.get('id')
    accountInfo = data.get('acc')

    #db.validateCombo(comboId)
    notifs.notify(comboId, accountInfo)
*/
/*
@socketio.event
def settings():
    print('Sending settings')
    emit('settings', {
        'threads': config.ThreadsPerNode,
        'proxies': []
    })
*/

macro_rules! get {
    (with $json:ident from $socket:ident get $($(and)? $item:literal $($fn:ident)?),*) => {
        ($(
            match $json.get($item)$(.and_then(Value::$fn))? {
                Some(val) => val,
                None => {
                    let resp = json!({
                        "errors":[
                            "Could not find key in key-value pair \"keyspace\""
                        ]
                    });
                    log_err($socket.emit("errors", resp));
                    return;
                },
            }
        ),*)
    }
}

fn log_err<E: Error>(res: Result<(), E>) {
    match res {
        Ok(_) => {},
        Err(err) => eprintln!("Error occurred in socket: {:#?}", err),
    }
}

fn settings(payload: Payload, socket: RawClient) {
    let payload = match payload_to_json(&payload) {
        Ok(val) => val,
        Err(_) => {
            let resp = json!({
                "errors": ["Unauthorized"]
            });
            log_err(socket.emit("errors", resp));
            return;
        },
    };

    let auth = get!(with payload from socket get "auth" as_str);

    if !APIKEYS.contains(&auth) {
        return;
    }

    log_err(socket.emit("settings", json!({
        "batch_size": CONFIG.settings.batch_size,
        "workers_in_rest_api": CONFIG.settings.rest_threads
    })));
}

fn connect(payload: Payload, socket: RawClient) {
    let payload = match payload_to_json(&payload) {
        Ok(val) => val,
        Err(_) => {
            let resp = json!({
                "errors": ["Unauthorized"]
            });
            log_err(socket.emit("errors", resp));
            return;
        },
    };

    let auth = get!(with payload from socket get "auth" as_str);

    if APIKEYS.contains(&auth) {
        log_err(socket.emit("authenticated", json!({})));
    }

    println!("Node connected");
}

fn payload_to_json(payload: &Payload) -> Result<Value, Value> {
    let payload = match payload {
        Payload::Binary(bytes) => bytes.iter().map(|ch| *ch as char).collect::<String>(),
        Payload::String(str) => str.to_owned(),
    };

    match json::from_str::<Value>(&payload) {
        Ok(val) => Ok(val),
        Err(err) => {
            let resp = json!({
                "errors": [
                    "error when parsing json",
                    err.to_string()
                ]
            });
            Err(resp)
        },
    }
}

fn validate(payload: Payload, socket: RawClient) {
    let payload = match payload_to_json(&payload) {
        Ok(val) => val,
        Err(err) => {
            log_err(socket.emit("errors", err));
            return;
        },
    };

    let (account_info, combo_id, auth) = get!(with payload from socket get "acc", "id" as_str, and "auth" as_str);

    if !APIKEYS.contains(&auth) {
        return;
    }

    notify(combo_id, account_info);
}

pub fn init(session: &'static Session) {
    let get_keyspace = |keyspace: &str| Keyspace::from(session, &*keyspace);

    let fetch = move |payload: Payload, socket: RawClient| {
        let payload = match payload_to_json(&payload) {
            Ok(val) => val,
            Err(err) => {
                log_err(socket.emit("errors", err));
                return;
            },
        };

        let (keyspace, auth) = get!(with payload from socket get "keyspace" as_str, "auth" as_str);

        if !APIKEYS.contains(&auth) {
            return;
        }

        let mut keyspace = get_keyspace(keyspace);

        let data = keyspace.fetch_email().1;

        log_err(socket.emit("combo", data));
    };


    let invalidate = move |payload: Payload, socket: RawClient| {
        let payload = match payload_to_json(&payload) {
            Ok(val) => val,
            Err(err) => {
                log_err(socket.emit("errors", err));
                return;
            },
        };

        let (id, keyspace, auth) = get!(with payload from socket get "uuid" as_str, "keyspace" as_str, and "auth" as_str);

        if !APIKEYS.contains(&auth) {
            return;
        }

        let keyspace = get_keyspace(keyspace);

        keyspace.invalidate_combo(id.to_owned());
    };

    let disconnect = |_: Payload, _: RawClient| {
        println!("Node disconnected")
    };

    ClientBuilder::new("http://localhost:4200/")
        .on("fetch", fetch)
        .on("invalid", invalidate)
        .on("validate", validate)
        .on("settings", settings)
        .on("connect", connect)
        .on("disconnect", disconnect)
        .namespace("/")
        .connect()
        .expect("could not start server");
}


