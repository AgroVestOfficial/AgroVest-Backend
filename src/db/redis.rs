use redis::aio::ConnectionManager;
use redis::AsyncCommands;

pub async fn get_cached<T: serde::de::DeserializeOwned>(
    redis: &mut ConnectionManager,
    key: &str,
) -> Option<T> {
    let data: Option<String> = redis.get(key).await.ok()?;
    data.and_then(|d| serde_json::from_str(&d).ok())
}

pub async fn set_cached<T: serde::Serialize>(
    redis: &mut ConnectionManager,
    key: &str,
    value: &T,
    ttl_secs: u64,
) -> anyhow::Result<()> {
    let json = serde_json::to_string(value)?;
    redis.set_ex::<_, _, ()>(key, json, ttl_secs).await?;
    Ok(())
}

pub async fn invalidate_pattern(
    redis: &mut ConnectionManager,
    pattern: &str,
) -> anyhow::Result<()> {
    let keys: Vec<String> = redis.keys(pattern).await?;
    if !keys.is_empty() {
        redis.del::<_, ()>(&keys).await?;
    }
    Ok(())
}
