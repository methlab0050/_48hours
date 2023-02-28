use std::{sync::atomic::{AtomicUsize, Ordering}};

use cassandra_cpp::{Session, stmt, Result, Ssl, SslVerifyFlag, Cluster, Error, UuidGen};
use rand::{thread_rng, Rng};
use rocket::{request::{FromRequest, Outcome}, async_trait, tokio::{self, fs::{read_to_string, metadata}}, http::Status, serde::{json::{json, Value}, Serialize}};
use tokio::main as sync;

use crate::config::CONFIG;

pub struct Combo {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct FullCombo {
    email: String,
    password: String,
    params: String,
    id: String,
}

#[derive(Serialize)]
pub struct FullComboPayload {
    data: Option<Vec<FullCombo>>,
    errors: Option<Vec<String>>,
}

pub struct Keyspace<'a> {
    session: &'a Session,
    keyspace: &'a str,
    index: &'a mut AtomicUsize
}

static mut DISCORD_INDEX: AtomicUsize = AtomicUsize::new(0);
static mut EMAIL_INDEX: AtomicUsize = AtomicUsize::new(0);
static mut VALID_EMAIL_INDEX: AtomicUsize = AtomicUsize::new(0);

#[async_trait]
impl<'r> FromRequest<'r> for Keyspace<'r> {
    type Error = std::convert::Infallible;

    async fn from_request(request: &'r rocket::Request<'_>) ->  Outcome<Self, Self::Error> {
        
        let session = request.rocket().state::<Session>().unwrap();
        let keyspace = match request.uri().path().segments().last() {
            Some(last_seg) => last_seg,
            None => "email"
        };
        let keyspace = Keyspace::from(session, keyspace);

        Outcome::Success(keyspace)
    }
}

impl<'a> Keyspace<'a> {
    pub fn from(session: &'a Session, keyspace: &str) -> Self {
        let (keyspace, index) = unsafe {
            match keyspace {
                "discord" => ("discord", &mut DISCORD_INDEX),
                "valid" => ("valid", &mut VALID_EMAIL_INDEX),
                "email" | _ => ("email", &mut EMAIL_INDEX),
            }
        };
        Keyspace {
            keyspace, session, index
        }
    }

    fn get_table(&mut self) -> (String, usize) {
        let new_index = if self.index.get_mut() >= &mut CONFIG.cassandra.total_tables.clone() {
            0
        } else {
            self.index.get_mut().clone() + 1
        };

        self.index.swap(new_index, Ordering::Relaxed);

        (format!("{}.t{}", self.keyspace, new_index), new_index)
    }

    pub async fn create_table_if_not_exists(&self) {
        let statement = stmt!(&format!(
            "CREATE KEYSPACE IF NOT EXISTS {}
        WITH REPLICATION = {{ 'class' : 'SimpleStrategy', 'replication_factor' : 1 }}",
            self.keyspace
        ));

        match self.session.execute(&statement).await {
            Err(err) => {
                eprintln!("Error: failed to create keyspace {:?} {}", self.keyspace, err);
                return;
            }
            Ok(_) => println!("Created keyspace {}", self.keyspace),
        }

        for i in 0..CONFIG.cassandra.total_tables {
            let table_name = format!("{}.t{}", self.keyspace, i);
            println!("Creating table {table_name}...");
            let statement = stmt!(&format!("CREATE TABLE IF NOT EXISTS {table_name} (id text PRIMARY KEY, email text, passw text, lastcheck timestamp, p text)"));
            match self.session.execute(&statement).await {
                Ok(_) => println!("Created table {}", table_name),
                Err(err) => eprintln!("Failed to create table {table_name}: {err}"),
            }
        }

        println!("Created all tables")
    }

    fn get_table_by_uuid(&self, id: String) -> String {
        format!("{}.t{}", self.keyspace, id.split("-").nth(0).unwrap())
    }

    async fn add(&self, query: String) -> Option<Error> {
        let statement = stmt!(&query);
        self.session.execute(&statement).await.err()
    }

    #[sync]
    pub async fn add_combos(&mut self, combos: Vec<Combo>, params: &str) -> Option<Vec<String>> {
        let mut query = "BEGIN BATCH\n".to_string();
        let mut added = 0;

        let mut errors = Vec::new();

        for combo in combos {
            let (table, i) = self.get_table();
            query += &format!("INSERT INTO {} (email, passw, lastcheck, p, id) VALUES ('{}', '{}', 0, '{}', '{}')\n", table, sanitize(combo.email), sanitize(combo.password), sanitize(params), generate_id(i));
            added += 1;

            if query.len() > 49500 {
                match self.add(query.clone()).await {
                    Some(err) => errors.push(format!("failed to add {added} combos into {}: {err}", self.keyspace)),
                    None => println!("added {added} combos"),
                }
                added = 0;
                query = "BEGIN BATCH\n".to_string();
            }
        }
        match self.add(query.clone()).await {
            Some(err) => errors.push(format!("failed to add {added} combos into {}: {err}", self.keyspace)),
            None => println!("added {added} combos"),
        }

        if errors.is_empty() {
            None
        } else {
            Some(errors)
        }
    }

    #[sync]
    pub async fn fetch_email(&mut self) -> (Status, Value) {
        let (table, _) = self.get_table();

        let statement = format!(
            "SELECT * FROM {table}
        LIMIT {}
        ALLOW FILTERING
        ", CONFIG.settings.batch_size
        );
        let statement = stmt!(&statement);

        let emails = match self.session.execute(&statement).await {
            Ok(val) => val,
            Err(err) => return (Status::InternalServerError, json!({
                "errors": err.to_string()
            })),
        };
        let emails = emails.iter().collect::<Vec<_>>();

        let mut data = Vec::new();
        let mut errors = Vec::new();

        for i in 0..CONFIG.settings.batch_size {
            let Some(row) = emails.get(i) else { continue; };
            
            macro_rules! get {
                ($name:literal) => {
                    match row.get_column_by_name($name) {
                        Ok(val) => val.to_string(),
                        Err(err) => {
                            errors.push(format!("{}", err));
                            continue;
                        },
                    }
                };
            }

            let email = get!("email");
            let password = get!("passw");
            let params = get!("p");
            let id = get!("id");

            let combo = FullCombo { email, password, params, id: id.to_owned() };
            data.push(combo);
            self.invalidate_combo(id);
        }
        
        match (data.is_empty(), errors.is_empty()) {
            (false, false) => {
                (Status::Ok, json!({
                    "errors": errors,
                    "data": data,
                }))
            }
            (false, true) => {
                (Status::Ok, json!({
                    "data": data,
                }))
            },
            (true, false) => {
                (Status::InternalServerError, json!({
                    "errors": errors
                }))
            }
            (true, true) => {
                (Status::ImATeapot, json!("There're no errors... but no data either.... What???? This error should be infalible, but just to be sure, I'm putting a very serious 418 status code on it"))
            }
        }
    }

    #[sync]
    pub async fn invalidate_combo(&self, id: String) -> Option<Error> {
        let table = self.get_table_by_uuid(id.clone());
        let statement = format!("DELETE FROM {table} WHERE id = '{}'", sanitize(id));
        let statement = stmt!(&statement);
        self.session.execute(&statement).await.err()
    }
}

fn sanitize<S: AsRef<str>>(input: S) -> String {
    input.as_ref().replace("'", "''")
}

fn generate_id(i: usize) -> String {
    format!("{}-{}", i, &UuidGen::default().gen_random().to_string().replace("-", "")[..8])
}

pub async fn connect() -> Result<Session> {

    let config = CONFIG.cassandra;
    let mut cluster = Cluster::default();
    let session_builder = cluster
        .set_port(config.port)?
        .set_contact_points(&config.nodes[0])?
        .set_credentials(&config.username,&config.password)?;

    add_cert(session_builder).await;

    let session = session_builder
        .connect_async()
        .await?;
    
    unsafe { init(&session) };
    
    Ok(session)
}

async fn add_cert(session_builder: &mut Cluster) {
    if metadata("aws.cert").await.is_ok() {
        let Ok(cert) = read_to_string("aws.cert").await else { return; };

        let mut ssl = Ssl::default();
        let ssl = ssl.add_trusted_cert(&cert);
        let Ok(mut ssl) = ssl else { return; };
        ssl.set_verify_flags(&[SslVerifyFlag::PEER_CERT]);
        session_builder.set_ssl(&mut ssl);
    }
}

#[sync]
async unsafe fn init(session: &Session) {
    let keyspace_contexts = [
        ("discord", &mut DISCORD_INDEX), 
        ("valid", &mut VALID_EMAIL_INDEX), 
        ("email", &mut EMAIL_INDEX),
    ];

    for (keyspace, index) in keyspace_contexts {
        let keyspace = Keyspace { keyspace, session, index };
        keyspace.create_table_if_not_exists().await
    }

    DISCORD_INDEX = thread_rng().gen_range(0..CONFIG.cassandra.total_tables).into();
    EMAIL_INDEX = thread_rng().gen_range(0..CONFIG.cassandra.total_tables).into();
    VALID_EMAIL_INDEX = thread_rng().gen_range(0..CONFIG.cassandra.total_tables).into();
}

