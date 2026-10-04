use shared::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod bans;
mod ip_bans;
mod kick;
mod online;
mod operators;
mod overview;
mod whitelist;

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(overview::get::route))
        .routes(routes!(online::get::route))
        .routes(routes!(whitelist::add::route))
        .routes(routes!(whitelist::remove::route))
        .routes(routes!(whitelist::enabled::route))
        .routes(routes!(operators::add::route))
        .routes(routes!(operators::remove::route))
        .routes(routes!(bans::add::route))
        .routes(routes!(bans::remove::route))
        .routes(routes!(ip_bans::add::route))
        .routes(routes!(ip_bans::remove::route))
        .routes(routes!(kick::post::route))
        .with_state(state.clone())
}
