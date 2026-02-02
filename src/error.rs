use poise::serenity_prelude::prelude::SerenityError;
use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(transparent)]
    SerenityError {
        source: SerenityError
    }
}