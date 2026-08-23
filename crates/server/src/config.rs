pub struct Config {
    pub database_url: String,
    pub pepper: Vec<u8>,
    pub bind_addr: String,
}

impl Config {
    pub fn from_env() -> Self {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pepper = std::env::var("PASSWORD_PEPPER")
            .expect("PASSWORD_PEPPER must be set")
            .into_bytes();
        let bind_addr =
            std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());

        Self {
            database_url,
            pepper,
            bind_addr,
        }
    }
}
