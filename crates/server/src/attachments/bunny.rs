use crate::state::BunnyConfig;

/// Uploads a body (streamed from the local temp file the upload handler
/// already wrote, so a 2GB attachment never has to sit fully in memory) to
/// Bunny Storage under `storage_key`. The caller already knows the final
/// public URL is `{cdn_base_url}/{storage_key}` - this only has to land the
/// bytes at the matching path in the zone.
pub async fn upload(
    client: &reqwest::Client,
    bunny: &BunnyConfig,
    storage_key: &str,
    body: reqwest::Body,
) -> Result<(), reqwest::Error> {
    let url = format!(
        "https://{}/{}/{}",
        bunny.storage_region_host, bunny.storage_zone, storage_key
    );
    client
        .put(url)
        .header("AccessKey", bunny.storage_api_key.as_ref())
        .header("Content-Type", "application/octet-stream")
        .body(body)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

/// Best-effort delete - called from the retention sweep, where a stray
/// object outliving its DB row is a wasted few KB, not a correctness bug,
/// so failures are logged rather than propagated.
pub async fn delete(client: &reqwest::Client, bunny: &BunnyConfig, storage_key: &str) {
    let url = format!(
        "https://{}/{}/{}",
        bunny.storage_region_host, bunny.storage_zone, storage_key
    );
    if let Err(err) = client
        .delete(url)
        .header("AccessKey", bunny.storage_api_key.as_ref())
        .send()
        .await
    {
        tracing::warn!("failed to delete {storage_key} from Bunny storage: {err}");
    }
}

pub fn public_url(bunny: &BunnyConfig, storage_key: &str) -> String {
    format!("{}/{}", bunny.cdn_base_url, storage_key)
}
