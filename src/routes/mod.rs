use shared::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod kick;
mod lists;
mod online;
mod overview;
mod whitelist;

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(overview::get::route))
        .routes(routes!(online::get::route))
        .routes(routes!(lists::post::route))
        .routes(routes!(lists::delete::route))
        .routes(routes!(whitelist::put::route))
        .routes(routes!(kick::post::route))
        .with_state(state.clone())
}
