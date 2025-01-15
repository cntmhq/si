use axum::{
    async_trait,
    extract::{FromRequestParts, Host, OriginalUri},
    http::{request::Parts, Uri},
    RequestPartsExt as _,
};
use dal::DalContext;
use derive_more::{Deref, Into};

use crate::app_state::AppState;

use super::{internal_error, not_found_error, ErrorResponse};

#[derive(Clone, Debug, Deref, Into)]
pub struct HandlerContext(pub dal::DalContextBuilder);

#[async_trait]
impl FromRequestParts<AppState> for HandlerContext {
    type Rejection = ErrorResponse;

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let builder = state
            .services_context()
            .clone()
            .into_inner()
            .into_builder(state.for_tests());
        Ok(Self(builder))
    }
}

#[derive(Clone, Debug, Deref, Into)]
pub struct AssetSprayer(pub asset_sprayer::AssetSprayer);

#[async_trait]
impl FromRequestParts<AppState> for AssetSprayer {
    type Rejection = ErrorResponse;

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let asset_sprayer = state
            .asset_sprayer()
            .ok_or(not_found_error("openai not configured"))?;
        Ok(Self(asset_sprayer.clone()))
    }
}

#[derive(Clone, Debug, Deref, Into)]
pub struct PosthogClient(pub crate::app_state::PosthogClient);

#[async_trait]
impl FromRequestParts<AppState> for PosthogClient {
    type Rejection = ErrorResponse;

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self(state.posthog_client().clone()))
    }
}

#[derive(Clone, Debug, Deref, Into)]
pub struct Nats(pub si_data_nats::NatsClient);

#[async_trait]
impl FromRequestParts<AppState> for Nats {
    type Rejection = ErrorResponse;

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let services_context = state.services_context();
        Ok(Self(services_context.nats_conn().clone()))
    }
}

///
/// Provides a DalContext and a track() method to log the endpoint call.
///
/// Always used as part of an Authorization object (cannot be constructed).
///
#[derive(Clone)]
pub struct PosthogEventTracker {
    // These last three are so endpoints can do request tracking (they all do it the same way)
    pub posthog_client: crate::app_state::PosthogClient,
    pub original_uri: Uri,
    pub host: String,
}

impl PosthogEventTracker {
    pub fn track(
        &self,
        ctx: &DalContext,
        event_name: impl AsRef<str>,
        properties: serde_json::Value,
    ) {
        crate::tracking::track(
            &self.posthog_client,
            ctx,
            &self.original_uri,
            &self.host,
            event_name,
            properties,
        )
    }
}

#[async_trait]
impl FromRequestParts<AppState> for PosthogEventTracker {
    type Rejection = ErrorResponse;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Grab a few other things everybody needs (for tracking)
        let OriginalUri(original_uri) = parts.extract().await.map_err(internal_error)?;
        let Host(host) = parts.extract().await.map_err(internal_error)?;
        let PosthogClient(posthog_client) = parts.extract_with_state(state).await?;
        Ok(Self {
            posthog_client,
            original_uri,
            host,
        })
    }
}
