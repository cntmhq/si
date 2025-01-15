use axum::Router;

use crate::AppState;

pub mod change_sets;
pub mod workspaces;

pub fn routes(state: AppState) -> Router<AppState> {
    Router::new().nest(
        "/v0",
        Router::new().nest("/workspaces", workspaces::routes(state)),
    )
}
