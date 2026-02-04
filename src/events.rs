use std::sync::RwLock;

use poise::serenity_prelude::{self as serenity, CreateWebhook};
use serenity::{Webhook, json::json};

use crate::{CONFIG, Data, error::Error};

pub async fn acquire_webhook(
    ctx: &serenity::Context,
    data: &RwLock<Data>,
) -> Result<Webhook, Error> {
    let url = { CONFIG.read().unwrap().get_webhook().cloned() };
    match url {
        None => {
            let map = json!({"name": "SimDemocracy"});
            let webhook = {
                let parent = {
                    let thread = data.read().unwrap();
                    ctx.http.get_channel(thread.cfc_thread_id)
                }
                .await?
                .guild()
                .unwrap()
                .parent_id
                .unwrap();
                let webhooks = parent.webhooks(&ctx).await.unwrap();
                match webhooks
                    .into_iter()
                    .find(|w| w.name == Some("SimDemocracy Bot".to_string()))
                {
                    Some(w) => w,
                    None => {
                        parent
                            .create_webhook(&ctx, CreateWebhook::new("SimDemocracy Bot"))
                            .await?
                    }
                }
            };
            drop(map);
            {
                let mut lock = CONFIG.write().unwrap();
                lock.set_webhook(webhook.url().unwrap());
            }
            Ok(webhook)
        }
        Some(webhook) => Ok(Webhook::from_url(ctx, &webhook).await?),
    }
}
