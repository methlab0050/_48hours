pub struct Cassandra<S: AsRef<str>, const N: usize> {
    pub nodes: [S; N],
    pub username: S,
    pub password: S,
    pub port: u16,
    pub total_tables: usize,
}

pub const CASSANDRA: Cassandra<&str, 2> = Cassandra {
    nodes: ["23.239.31.117", "45.79.8.54"],
    username: "cassandra",
    password: "O#o`8|-=c2tT3~<lEX",
    port: 9042,
    total_tables: 10
};

// Node connections
pub const APIKEYS: [&str; 2] = ["ABCDEFGHIJKLMNBOPQRSTUVWXYZ", "amazonspotinstance997152"];
pub const THREADSPERNODE: usize = 4;

// logging
pub const DISCORDWEBHOOK: &str = ""; //'https://discord.com/api/webhooks/1026000711055589406/JOeLgSAy6DR7rAri3UMr7MA3xM866TNRmxyEImcvVF2GFXG8mG8m83sVI7LgwVqpGoCD'
pub const TELEGRAMTOKEN: &str = ""; 
pub const TELEGRAMCHATID: &str = ""; 
