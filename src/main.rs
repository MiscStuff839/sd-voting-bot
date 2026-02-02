use ordinal::ToOrdinal;
use poise::{
    CreateReply, FrameworkContext,
    serenity_prelude::{
        self as serenity, CacheHttp, ChannelId, ChannelType, CreateActionRow, CreateButton, CreateEmbed, CreateMessage, CreateThread,
        CreateWebhook, ExecuteWebhook, FullEvent, Webhook, json::json,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    fs::read_to_string,
    sync::RwLock,
    vec,
};

use crate::{data::*, error::Error};

mod data;
mod error;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct Config {
    token: String,
    webhook: Option<String>,
}

#[derive(Debug)]
struct Data {
    cfc_thread_id: RwLock<ChannelId>,
    webhook: RwLock<Option<String>>,
}

type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() {
    let cfg = match toml::from_str::<Config>(
        &read_to_string(std::env::current_dir().unwrap().join("config.toml"))
            .expect("Unable to find a config file"),
    ) {
        Ok(val) => val,
        Err(err) => panic!("TOML deserialisation error: {}", err),
    };
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![cfc_senate()],
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    cfc_thread_id: RwLock::new(ChannelId::new(1)),
                    webhook: RwLock::new(cfg.webhook),
                })
            })
        })
        .build();

    let intents = serenity::GatewayIntents::non_privileged();
    let client = serenity::ClientBuilder::new(&cfg.token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap();
}

async fn event_handler(
    ctx: &serenity::Context,
    event: &FullEvent,
    _: FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    if let FullEvent::InteractionCreate { interaction } = event {
        if let serenity::Interaction::Component(component_interaction) = interaction {
            if component_interaction.data.custom_id.parse::<u32>().is_ok()
                && data.cfc_thread_id.read().unwrap().get() != 1
            {
                let cfc = execute_modal_generic(
                    ctx,
                    |resp| component_interaction.create_response(ctx, resp),
                    format!("{}_cfc", component_interaction.user.id),
                    None::<SenateCFCModal>,
                    None,
                )
                .await?;
                match cfc {
                    Some(cfc) => {
                        let webhook = {
                            let lock = data.webhook.read().unwrap();
                            lock.as_ref().cloned()
                        };
                        match webhook {
                            None => {
                                let map = json!({"name": "SimDemocracy"});
                                let webhook = {
                                    let parent = {
                                        let thread = data.cfc_thread_id.read().unwrap();
                                        ctx.http().get_channel(*thread)
                                    }
                                    .await?;

                                    parent.guild().unwrap().parent_id.unwrap().create_webhook(
                                        &ctx,
                                        CreateWebhook::new("SimDemocracy Bot"),
                                    )
                                }
                                .await
                                .unwrap();
                                drop(map);
                                {
                                    let mut lock = data.webhook.write().unwrap();
                                    *lock = Some(webhook.url().unwrap());
                                }
                                {
                                    let thread = data.cfc_thread_id.read().unwrap();
                                    webhook.execute(
                                        ctx,
                                        false,
                                        ExecuteWebhook::new()
                                            .content(format!(
                                                "<@{}>\n{cfc}",
                                                component_interaction.user.id.get()
                                            ))
                                            .avatar_url(
                                                component_interaction.user.avatar_url().unwrap_or(
                                                    component_interaction.user.default_avatar_url(),
                                                ),
                                            )
                                            .username(component_interaction.user.display_name())
                                            .in_thread(*thread),
                                    )
                                }
                                .await?;
                            }
                            Some(webhook) => {
                                let webhook = Webhook::from_url(ctx, &webhook).await?;
                                webhook
                                    .execute(
                                        ctx,
                                        false,
                                        ExecuteWebhook::new()
                                            .content(format!(
                                                "<@{}>\n{cfc}",
                                                component_interaction.user.id.get()
                                            ))
                                            .avatar_url(
                                                component_interaction.user.avatar_url().unwrap_or(
                                                    component_interaction.user.default_avatar_url(),
                                                ),
                                            )
                                            .username(component_interaction.user.display_name()),
                                    )
                                    .await?;
                            }
                        }
                    }
                    None => {
                        component_interaction.user.dm(&ctx, CreateMessage::new().add_embed(CreateEmbed::new().description("The form you submitted was empty. Resubmit for your cfc to be registered"))).await?;
                    }
                }
            }
        }
    }
    Ok(())
}

#[poise::command(slash_command)]
async fn cfc_senate(
    ctx: Context<'_>,
    #[description = "The term which this cfc is for"] term_number: u32,
) -> Result<(), Error> {
    ctx.send(CreateReply::default().embed(CreateEmbed::new().title(format!("{} Senate Call for Candidates", term_number.to_ordinal())).description("
Lorem ipsum dolor sit amet, consectetur adipiscing elit. Phasellus rhoncus iaculis rhoncus. Pellentesque risus 
ipsum, porttitor sed metus quis, condimentum fermentum nisl. 
Cras condimentum massa eget lacus dignissim, a ullamcorper massa laoreet. Sed varius justo id venenatis gravida. 
In porta augue at urna iaculis, ac luctus nunc tristique. Suspendisse potenti. Etiam mollis id sem et mollis. 
Etiam eu ultricies magna. Aliquam nec justo rhoncus, volutpat magna at, auctor risus. Phasellus sit amet eros ex. 
Aliquam erat volutpat."))
.components(vec![CreateActionRow::Buttons(vec![CreateButton::new(term_number.to_string()).label("Submit CFC")])])).await?;
    let channel = ctx
        .guild_channel()
        .await
        .unwrap()
        .create_thread(
            &ctx,
            CreateThread::new(format!(
                "{} Senate Call for Candidates",
                term_number.to_ordinal()
            ))
            .kind(ChannelType::PublicThread),
        )
        .await?
        .id;
    let mut lock = ctx.data().cfc_thread_id.write().unwrap();
    *lock = channel;

    Ok(())
}
