use std::fmt::Display;

use bitcode::{Decode, Encode};
use poise::{Modal, serenity_prelude::{self as serenity, MessageId, UserId}};

#[derive(Debug, Modal, Decode, Encode)]
#[name = "CFC Application"]
pub struct SenateCFCModal {
    #[name = "Reddit Username (with u/)"]
    reddit_user: Option<String>,
    #[name = "Political Party/Coalition"]
    party: String,
    #[name = "CFC Statement"]
    #[paragraph]
    cfc: String,
}

impl Display for SenateCFCModal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "/{} | {}\n\n{}",
            self.reddit_user
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("No Reddit Account"),
            self.party,
            self.cfc
        )
    }
}

pub async fn execute_modal_generic<
    M: Modal,
    F: std::future::Future<Output = Result<(), serenity::Error>>,
>(
    ctx: &serenity::Context,
    create_interaction_response: impl FnOnce(serenity::CreateInteractionResponse) -> F,
    modal_custom_id: String,
    defaults: Option<M>,
    timeout: Option<std::time::Duration>,
) -> Result<Option<M>, serenity::Error> {
    // Send modal
    create_interaction_response(M::create(defaults, modal_custom_id.clone())).await?;

    // Wait for user to submit
    let response = serenity::collector::ModalInteractionCollector::new(&ctx.shard)
        .filter(move |d| d.data.custom_id == modal_custom_id)
        .timeout(timeout.unwrap_or(std::time::Duration::from_secs(3600)))
        .await;
    let response = match response {
        Some(x) => x,
        None => return Ok(None),
    };

    // Send acknowledgement so that the pop-up is closed
    response
        .create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
        .await?;

    Ok(Some(
        M::parse(response.data.clone()).map_err(serenity::Error::Other)?,
    ))
}