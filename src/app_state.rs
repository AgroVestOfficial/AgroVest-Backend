use crate::config::AppConfig;
use crate::services::ipfs_service::IpfsService;
use sqlx::postgres::PgPoolOptions;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub redis: redis::aio::ConnectionManager,
    pub config: AppConfig,
    pub http_client: reqwest::Client,
    pub ipfs: IpfsService,
}

impl AppState {
    pub async fn new(config: AppConfig) -> anyhow::Result<Self> {
        let db = PgPoolOptions::new()
            .max_connections(20)
            .connect(&config.database_url)
            .await?;

        let redis_client = redis::Client::open(config.redis_url.clone())?;
        let redis = redis::aio::ConnectionManager::new(redis_client).await?;

        let http_client = reqwest::Client::new();

        let ipfs = IpfsService::new(
            http_client.clone(),
            config.pinata_api_key.clone(),
            config.pinata_secret_key.clone(),
            config.ipfs_gateway_url.clone(),
        );

        Ok(Self {
            db,
            redis,
            config,
            http_client,
            ipfs,
        })
    }
}
