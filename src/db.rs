use std::fs::metadata;

use cassandra_cpp::{Cluster, Session, Ssl, SslVerifyFlag, stmt};
use crate::config::CASSANDRA as cas;
use rand::{thread_rng, Rng};

fn init() {
}

async fn connect() -> cassandra_cpp::Result<Session> {
    if metadata("aws.cert").is_ok() {

        Ssl::default().set_verify_flags(&[SslVerifyFlag::PEER_CERT]);
    }
    
    let mut cluster = Cluster::default();
    cluster.set_port(cas.port)?
        .set_contact_points(cas.nodes[0])?
        .set_credentials(cas.username, cas.password)?
        // .set_ssl(ssl)
        .connect_async().await
}


fn get_table(index: &mut Option<usize>) -> (String, usize) {
    let used_i = match index {
        Some(ref i) => {
            if i >= &cas.total_tables {
                index.insert(0).clone()
            } else {
                index.insert(i + 1).clone()
            }
        },
        None => thread_rng().gen_range(0..cas.total_tables)
    };

    (format!("combos.t{used_i}"), used_i)
}

async fn create_table_if_not_exists(session: &Session) {
    let statement = stmt!("CREATE KEYSPACE IF NOT EXISTS combos
    WITH REPLICATION = { 'class' : 'SimpleStrategy', 'replication_factor' : 1 }");

    session.execute(&statement);

    for i in 0..cas.total_tables {
        let table_name = format!("combos.t{i}");
        println!("Creating table {table_name}...");
        let statement = stmt!(&format!("CREATE TABLE IF NOT EXISTS {table_name} (id text PRIMARY KEY, email text, passw text, lastcheck timestamp, p text)"));
        session.execute(&statement);
    }
    // print('Created all tables')
    println!("Created all tables")
}

fn get_table_by_uuid(id: usize) -> String {
    format!("combos.t{}", id.to_string().split("-").nth(0).unwrap())
}

struct Combo {
    email: String,
    password: String,
}


fn add_combos(index: &mut Option<usize>, session: &Session, combos: Vec<Combo>, params: &str) {
    fn add(session: &Session, query: String) {
        let statement = stmt!(&query);
        session.execute(&statement);
    }
    let mut query = "BEGIN BATCH\n".to_string();
    let mut added = 0;
    //#print('Adding ' + str(added) + ' combos...')
    //^put this somewhere

    for combo in combos {
        let (table, i) = get_table(index);
        query += &format!("INSERT INTO {} (email, passw, lastcheck, p, id) VALUES ('{}', '{}', 0, '{}', '{}')\n", table, sanitize(combo.email), sanitize(combo.password), sanitize(params), generate_id(i));
        added += 1;

        if query.len() > 49500 {
            add(session, query.clone());
            added = 0;
            query = "BEGIN BATCH\n".to_string();
        }
    }
    add(session, query.clone());
}

fn add_email(session: &Session, index: &mut Option<usize>, email: String, password: String, params: &str) {
    let (table, i) = get_table(index);
    let statement = format!("INSERT INTO {table} (email, passw, lastcheck, p, id)
        VALUES ({email}, {password}, {0}, {params}, {})", generate_id(i));
    let statement = stmt!(&statement);
    session.execute(&statement);
}

fn sanitize<S: AsRef<str>>(input: S) -> String {
    input.as_ref().replace("'", "''")
}

struct HAPE {
    email: String,
    password: String,
    params: String,
    id: usize,
}

pub async fn fetch_email(session: &Session, index: &mut Option<usize>) {
    let (table, _) = get_table(index);

    {
        let statement = format!("SELECT * FROM {table}
        LIMIT 1000
        ALLOW FILTERING
        ");
        let statement = stmt!(&statement);

        let emails = session.execute(&statement)
            .await
            .unwrap();
        let emails = emails.iter().collect::<Vec<_>>();

        for i in 0..1000 {
            let res = emails.get(i).unwrap();
            let get = |gg: &str| res.get_column_by_name(gg).unwrap();


        }
        
    }

    todo!()
}

fn invalidate_combo() {}

fn get_discord_table() {}

fn create_discord_table_if_not_exists() {}

fn get_discord_table_by_uuid() {}

fn add_discord_email() {}

fn generate_id(_d: usize) -> String {
    todo!()
}

fn add_discord_combos() {}

fn invalidate_discord_combo() {}

fn fetch_discord_email() {}



fn get_valid_table() {}

fn valid_email_create_table_if_not_exists() {}

fn get_valid_table_by_uuid() {}

fn add_valid_email() {}

fn add_valid_email_combo() {}

fn invalidate_email_combo() {}

fn fetch_valid_email() {}


