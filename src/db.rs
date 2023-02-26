use std::{sync::atomic::{AtomicUsize, Ordering}, fs::metadata};

use cassandra_cpp::{Session, stmt, Result, Ssl, SslVerifyFlag, Cluster, Error};
use rand::{thread_rng, Rng};
use rocket::{request::{FromRequest, Outcome}, async_trait, tokio, http::Status, serde::{json::{json, Value}, Serialize}};
use tokio::main as sync;

use crate::config::CASSANDRA;

type Uuid = String;
type Inet = String;

pub struct Combo {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct FullCombo {
    email: CassandraValue,
    password: CassandraValue,
    params: CassandraValue,
    id: CassandraValue,
}

#[derive(Serialize)]
pub struct FullComboPayload {
    data: Option<Vec<FullCombo>>,
    errors: Option<Vec<String>>,
}

#[derive(Serialize)]
pub enum CassandraValue {
    Bytes(Vec<u8>),
    Float32(f32),
    Float64(f64),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    UnsignedInt(u32),
    String(String),
    Inet(Inet),
    Uuid(Uuid),
    Unknown,
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
        let rr = match request.uri().path().segments().last() {
            Some(last_seg) => last_seg,
            None => "email"
        };
        let (keyspace, index) = unsafe {
            match rr {
                "discord" => ("discord", &mut DISCORD_INDEX),
                "valid" => ("valid", &mut VALID_EMAIL_INDEX),
                "email" | _ => ("email", &mut EMAIL_INDEX),
            }
        };
        let full_keyspace = Keyspace {
            keyspace, session, index
        };
        
        Outcome::Success(full_keyspace)
        // ;todo!()
    }
}

impl<'a> Keyspace<'a> {
    fn get_table(&mut self) -> (String, usize) {
        let new_index = if self.index.get_mut() >= &mut CASSANDRA.total_tables {
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

        for i in 0..CASSANDRA.total_tables {
            //combos.t
            let table_name = format!("{}.t{}", self.keyspace, i);
            println!("Creating table {table_name}...");
            let statement = stmt!(&format!("CREATE TABLE IF NOT EXISTS {table_name} (id text PRIMARY KEY, email text, passw text, lastcheck timestamp, p text)"));
            match self.session.execute(&statement).await {
                Ok(_) => println!("Created table {}", table_name),
                Err(err) => eprintln!("Failed to create table {table_name}: {err}"),
            }
        }
        // print('Created all tables')
        println!("Created all tables")
    }

    fn get_table_by_uuid(&self, id: Uuid) -> String {
        format!("{}.t{}", self.keyspace, id.split("-").nth(0).unwrap())
    }

    async fn add(&self, query: String, added: i32) -> Option<Error> {
        let statement = stmt!(&query);
        self.session.execute(&statement).await.err()
    }

    #[sync]
    pub async fn add_combos(&mut self, combos: Vec<Combo>, params: &str) -> Option<Vec<String>> {
        let mut query = "BEGIN BATCH\n".to_string();
        let mut added = 0;
        //#print('Adding ' + str(added) + ' combos...')
        //^put this somewhere

        let mut errors = Vec::new();

        for combo in combos {
            let (table, i) = self.get_table();
            query += &format!("INSERT INTO {} (email, passw, lastcheck, p, id) VALUES ('{}', '{}', 0, '{}', '{}')\n", table, sanitize(combo.email), sanitize(combo.password), sanitize(params), generate_id(i));
            added += 1;

            if query.len() > 49500 {
                if let Some(err) = self.add(query.clone(), added).await {
                    errors.push(format!("failed to add {added} combos into {}: {err}", self.keyspace))
                }
                added = 0;
                query = "BEGIN BATCH\n".to_string();
            }
        }
        if let Some(err) = self.add(query.clone(), added).await {
            errors.push(format!("failed to add {added} combos into {}: {err}", self.keyspace))
        }

        if errors.is_empty() {
            None
        } else {
            Some(errors)
        }
    }

    async fn add_email(&mut self, Combo{ email, password }: Combo, params: &str,) {
        let (table, i) = self.get_table();
        let statement = format!(
            "INSERT INTO {table} (email, passw, lastcheck, p, id)
            VALUES ({email}, {password}, {0}, {params}, {})",
            generate_id(i)
        );
        let statement = stmt!(&statement);
        self.session.execute(&statement).await.unwrap();
    }

    #[sync]
    pub async fn fetch_email(&mut self) -> (Status, Value) {
        let (table, _) = self.get_table();

        let statement = format!(
            "SELECT * FROM {table}
        LIMIT 1000
        ALLOW FILTERING
        "
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

        for i in 0..1000 {
            let Some(row) = emails.get(i) else { continue; };
            
            macro_rules! get {
                ($name:literal) => {
                    match row.get_column_by_name($name).and_then(convert) {
                        Ok(val) => val,
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

            let combo = FullCombo { email, password, params, id };
            data.push(combo);
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
                (Status::ImATeapot, json!("There're no errors... but no data either.... What???? This error should be infalible, but just to be sure, I'm putting a 418 status code on it"))
            }
        }
    }

    #[sync]
    pub async fn invalidate_combo(&self, id: Uuid) -> Option<Error> {
        let table = self.get_table_by_uuid(id.clone());
        let statement = format!("DELETE FROM {table} WHERE id = '{}'", sanitize(id));
        let statement = stmt!(&statement);
        self.session.execute(&statement).await.err()
    }
}

fn sanitize<S: AsRef<str>>(input: S) -> String {
    input.as_ref().replace("'", "''")
}

fn generate_id(i: usize) -> Uuid {
    let r = &uuid::Uuid::new_v4().to_string().replace("-", "")[..8];
    format!("{}-{}", i, r)
}

async fn connect() -> Result<Session> {
    if metadata("aws.cert").is_ok() {
        Ssl::default().set_verify_flags(&[SslVerifyFlag::PEER_CERT]);
    }

    let mut cluster = Cluster::default();
    cluster
        .set_port(CASSANDRA.port)?
        .set_contact_points(CASSANDRA.nodes[0])?
        .set_credentials(CASSANDRA.username, CASSANDRA.password)?
        // .set_ssl(ssl)
        .connect_async()
        .await
}

#[sync]
pub async unsafe fn init() -> Session {
    let session = connect().await.expect("");

    let keyspace_contexts = [
        ("discord", &mut DISCORD_INDEX), 
        ("valid", &mut VALID_EMAIL_INDEX), 
        ("email", &mut EMAIL_INDEX),
    ];

    for (keyspace, index) in keyspace_contexts {
        let keyspace = Keyspace { keyspace, session: &session, index };
        keyspace.create_table_if_not_exists().await
    }

    DISCORD_INDEX = thread_rng().gen_range(0..CASSANDRA.total_tables).into();
    EMAIL_INDEX = thread_rng().gen_range(0..CASSANDRA.total_tables).into();
    VALID_EMAIL_INDEX = thread_rng().gen_range(0..CASSANDRA.total_tables).into();

    session
}

fn convert(val: cassandra_cpp::Value) -> Result<CassandraValue> {
    macro_rules! _if {
        ($($_fn:ident, $_type:ident $(,$otherfn:ident)?);*) => {
            $(
                if let Ok(x) = val.$_fn() {
                    return Ok(CassandraValue::$_type(x$(.$otherfn())?));
                }
            )*
        };
    }

    _if!(
        get_f32, Float32;
        get_f64, Float64;
        get_i8, Int8;
        get_i16, Int16;
        get_i32, Int32;
        get_i64, Int64;
        get_u32, UnsignedInt;
        get_string, String;
        get_inet, Inet, to_string;
        get_uuid, Uuid, to_string;
        get_bytes, Bytes, to_vec
    );
    let err = Error::from_kind("could not read value in table as acceptable type".into());
    
    Err(err)
}
