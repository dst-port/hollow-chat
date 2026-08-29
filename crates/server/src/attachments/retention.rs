use std::time::Duration;

use sqlx::PgPool;
use uuid::Uuid;

use crate::attachments::bunny;
use crate::state::BunnyConfig;

const SWEEP_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// Spawns a background sweep that deletes the stored bytes (local disk or
/// Bunny, whichever the attachment landed on) of message/DM attachments
/// older than `retention_days`. Never touches avatars, banners, server
/// icons, custom emoji, or profile widget images - those are long-lived
/// assets, not chat history, and are excluded by the query below regardless
/// of age.
///
/// The `attachments` row itself is kept (with `purged_at` set) rather than
/// deleted, so the message it's attached to keeps rendering - just as an
/// expired attachment instead of a broken one.
pub fn spawn(
    pool: PgPool,
    attachments_dir: String,
    bunny: Option<BunnyConfig>,
    http_client: reqwest::Client,
    retention_days: u32,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(SWEEP_INTERVAL);
        loop {
            interval.tick().await;
            if let Err(err) =
                sweep_once(&pool, &attachments_dir, bunny.as_ref(), &http_client, retention_days).await
            {
                tracing::error!("attachment retention sweep failed: {err}");
                crate::telegram::notify(format!("attachment retention sweep failed: {err}"));
            }
        }
    });
}

async fn sweep_once(
    pool: &PgPool,
    attachments_dir: &str,
    bunny_config: Option<&BunnyConfig>,
    http_client: &reqwest::Client,
    retention_days: u32,
) -> Result<(), sqlx::Error> {
    let cutoff_days = retention_days as i32;
    let expired: Vec<(Uuid, String, bool)> = sqlx::query_as(
        "SELECT a.id, a.storage_key, a.on_cdn FROM attachments a \
         WHERE a.purged_at IS NULL \
           AND a.created_at < now() - make_interval(days => $1) \
           AND NOT EXISTS (SELECT 1 FROM users WHERE avatar_attachment_id = a.id OR banner_attachment_id = a.id) \
           AND NOT EXISTS (SELECT 1 FROM servers WHERE icon_attachment_id = a.id) \
           AND NOT EXISTS (SELECT 1 FROM custom_emoji WHERE attachment_id = a.id) \
           AND NOT EXISTS (SELECT 1 FROM profile_widgets WHERE image_attachment_id = a.id) \
         LIMIT 500",
    )
    .bind(cutoff_days)
    .fetch_all(pool)
    .await?;

    if expired.is_empty() {
        return Ok(());
    }

    let count = expired.len();
    for (id, storage_key, on_cdn) in expired {
        if on_cdn {
            if let Some(bunny_config) = bunny_config {
                bunny::delete(http_client, bunny_config, &storage_key).await;
            }
        } else {
            let path = std::path::Path::new(attachments_dir).join(&storage_key);
            if let Err(err) = tokio::fs::remove_file(&path).await {
                if err.kind() != std::io::ErrorKind::NotFound {
                    tracing::warn!("failed to remove expired attachment {storage_key}: {err}");
                    continue;
                }
            }
        }
        sqlx::query("UPDATE attachments SET purged_at = now() WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
    }

    tracing::info!("attachment retention sweep purged {count} attachment(s)");
    Ok(())
}
