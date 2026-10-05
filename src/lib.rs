use shared::{
    State,
    extensions::{Extension, ExtensionRouteBuilder},
};

mod console;
mod context;
mod files;
mod games;
mod lists;
mod model;
mod routes;
mod validate;

#[derive(Default)]
pub struct ExtensionStruct;

#[async_trait::async_trait]
impl Extension for ExtensionStruct {
    async fn initialize_router(
        &mut self,
        state: State,
        builder: ExtensionRouteBuilder,
    ) -> ExtensionRouteBuilder {
        builder.add_client_server_api_router(|router| {
            router.nest("/player-manager", routes::router(&state))
        })
    }
}
