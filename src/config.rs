#[derive(Clone, Debug)]
pub struct AppConfig {
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_expiration_hours: u64,
    pub pinata_api_key: String,
    pub pinata_secret_key: String,
    pub ipfs_backend: String,
    pub ipfs_gateway_url: String,
    pub soroban_rpc_url: String,
    pub farm_contract_address: String,
    pub investment_contract_address: String,
    pub escrow_contract_address: String,
    pub dao_contract_address: String,
    pub indexer_poll_interval_secs: u64,
    pub server_host: String,
    pub server_port: u16,
    pub cors_origins: Vec<String>,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        Ok(Self {
            database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".into()),
            jwt_secret: std::env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
            jwt_expiration_hours: std::env::var("JWT_EXPIRATION_HOURS")
                .unwrap_or_else(|_| "24".into())
                .parse()?,
            pinata_api_key: std::env::var("PINATA_API_KEY").unwrap_or_default(),
            pinata_secret_key: std::env::var("PINATA_SECRET_KEY").unwrap_or_default(),
            ipfs_backend: std::env::var("IPFS_BACKEND").unwrap_or_else(|_| "pinata".into()),
            ipfs_gateway_url: std::env::var("IPFS_GATEWAY_URL")
                .unwrap_or_else(|_| "https://gateway.pinata.cloud/ipfs".into()),
            soroban_rpc_url: std::env::var("SOROBAN_RPC_URL")
                .unwrap_or_else(|_| "https://soroban-testnet.stellar.org".into()),
            farm_contract_address: std::env::var("FARM_CONTRACT_ADDRESS").unwrap_or_default(),
            investment_contract_address: std::env::var("INVESTMENT_CONTRACT_ADDRESS")
                .unwrap_or_default(),
            escrow_contract_address: std::env::var("ESCROW_CONTRACT_ADDRESS").unwrap_or_default(),
            dao_contract_address: std::env::var("DAO_CONTRACT_ADDRESS").unwrap_or_default(),
            indexer_poll_interval_secs: std::env::var("INDEXER_POLL_INTERVAL_SECS")
                .unwrap_or_else(|_| "5".into())
                .parse()?,
            server_host: std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            server_port: std::env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".into())
                .parse()?,
            cors_origins: std::env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:3000".into())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
        })
    }
}
