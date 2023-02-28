pub struct Cassandra<'a> {
    pub nodes: &'a [&'a str],
    pub username: &'a str,
    pub password: &'a str,
    pub port: u16,
    pub total_tables: usize,
}

// Node connections
pub const APIKEYS: [&str; 2] = ["ABCDEFGHIJKLMNBOPQRSTUVWXYZ", "amazonspotinstance997152"];

// logging
pub const DISCORDWEBHOOK: Option<&str> = None; //'https://discord.com/api/webhooks/1026000711055589406/JOeLgSAy6DR7rAri3UMr7MA3xM866TNRmxyEImcvVF2GFXG8mG8m83sVI7LgwVqpGoCD'
pub const TELEGRAMTOKEN: Option<&str> = None; 
pub const TELEGRAMCHATID: Option<&str> = None; 

pub struct Config<'a> {
    pub settings: Settings<'a>,
    pub logging: Logging<'a>,
    pub cassandra: Cassandra<'a>,
}

pub struct Logging<'a> {
    pub discord_webhook: Option<&'a str>,
    pub telegram_token: Option<&'a str>,
    pub telegram_chatid: Option<&'a str>,
}

pub struct Settings<'a> {
    pub batch_size: usize,
    pub api_keys: &'a [&'a str],
    pub rest_threads: usize,
}


pub const CONFIG: Config<'static> = Config { 
    settings: Settings {
        batch_size: 1000, 
        api_keys: &["ABCDEFGHIJKLMNBOPQRSTUVWXYZ", "amazonspotinstance997152"], 
        rest_threads: 10,
    },
    logging: Logging {
        discord_webhook: None, //'https://discord.com/api/webhooks/1026000711055589406/JOeLgSAy6DR7rAri3UMr7MA3xM866TNRmxyEImcvVF2GFXG8mG8m83sVI7LgwVqpGoCD'
        telegram_token: None, 
        telegram_chatid: None, 
    },
    cassandra: Cassandra { 
        nodes: &["23.239.31.117", "45.79.8.54"],
        username: "cassandra",
        password: "O#o`8|-=c2tT3~<lEX",
        port: 9042,
        total_tables: 10
    }
};
