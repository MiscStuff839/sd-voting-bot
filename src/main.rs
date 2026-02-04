use bitcode::{decode, encode};
use lazy_static::lazy_static;
use ordinal::ToOrdinal;
use poise::{
    CreateReply, FrameworkContext,
    serenity_prelude::{
        self as serenity, CacheHttp, ChannelId, ChannelType, CreateActionRow, CreateButton,
        CreateEmbed, CreateMessage, CreateThread, CreateWebhook, EditWebhookMessage,
        ExecuteWebhook, FullEvent, MessageId, Webhook, json::json,
    },
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::read_to_string, sync::RwLock, vec};

use crate::{data::*, error::Error, events::acquire_webhook};

mod data;
mod error;
mod events;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct Config {
    token: String,
    webhook: Option<String>,
}

impl Config {
    fn set_webhook<T: Into<String>>(&mut self, webhook_url: T) {
        self.webhook = Some(webhook_url.into())
    }
    fn get_webhook(&self) -> Option<&String> {
        self.webhook.as_ref()
    }
}

#[derive(Debug)]
struct Data {
    cfc_thread_id: ChannelId,
    cfcs: HashMap<u64, (Vec<u8>, u64)>,
}

type Context<'a> = poise::Context<'a, RwLock<Data>, Error>;

lazy_static! {
    static ref CONFIG: RwLock<Config> = RwLock::new(
        match toml::from_str::<Config>(
            &read_to_string(std::env::current_dir().unwrap().join("config.toml"))
                .expect("Unable to find a config file"),
        ) {
            Ok(val) => val,
            Err(err) => panic!("TOML deserialisation error: {}", err),
        }
    );
}

#[tokio::main]
async fn main() {
    let cfg = CONFIG.read().unwrap();
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
                Ok(RwLock::new(Data {
                    cfc_thread_id: ChannelId::new(1),
                    cfcs: HashMap::new(),
                }))
            })
        })
        .build();

    let intents = serenity::GatewayIntents::non_privileged();
    let client = serenity::ClientBuilder::new(&cfg.token, intents)
        .framework(framework)
        .await;
    drop(cfg);
    client.unwrap().start().await.unwrap();
}

async fn event_handler(
    ctx: &serenity::Context,
    event: &FullEvent,
    _: FrameworkContext<'_, RwLock<Data>, Error>,
    data: &RwLock<Data>,
) -> Result<(), Error> {
    if let FullEvent::InteractionCreate { interaction } = event {
        if let serenity::Interaction::Component(component_interaction) = interaction {
            if component_interaction.data.custom_id.parse::<u32>().is_ok()
                && data.read().unwrap().cfc_thread_id.get() != 1
            {
                let webhook = acquire_webhook(ctx, data).await?;
                let thread_id = data.read().unwrap().cfc_thread_id;
                let msg = data
                    .read()
                    .unwrap()
                    .cfcs
                    .get(&component_interaction.user.id.get())
                    .cloned();
                let cfc = execute_modal_generic(
                    ctx,
                    |resp| component_interaction.create_response(ctx, resp),
                    format!("{}_cfc", component_interaction.user.id),
                    msg.as_ref()
                        .map(|(a, b)| (a, b))
                        .unzip()
                        .0
                        .map(|cfc| decode::<SenateCFCModal>(&cfc).unwrap()),
                    None,
                )
                .await?;
                match cfc {
                    Some(cfc) => {
                        let msg = if msg.is_none() {
                            webhook
                                .execute(
                                    ctx,
                                    true,
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
                                        .in_thread(thread_id),
                                )
                                .await?
                                .unwrap()
                        } else {
                            webhook
                                .edit_message(
                                    &ctx,
                                    MessageId::new(msg.unwrap().1),
                                    EditWebhookMessage::new()
                                        .content(format!(
                                            "<@{}>\n{cfc}",
                                            component_interaction.user.id.get()
                                        ))
                                        .in_thread(thread_id),
                                )
                                .await?
                        };
                        data.write().unwrap().cfcs.insert(
                            component_interaction.user.id.get(),
                            (encode(&cfc), msg.id.get()),
                        );
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
    let mut lock = ctx.data().write().unwrap();
    lock.cfc_thread_id = channel;
    Ok(())
}
