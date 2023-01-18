use axum::Json;

use dal::{Component, ComponentId, ComponentType, StandardModel, Visibility, WsEvent};
use serde::{Deserialize, Serialize};

use super::ComponentResult;
use crate::server::extract::{AccessBuilder, HandlerContext};
use crate::service::component::ComponentError;

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SetTypeRequest {
    pub component_id: ComponentId,
    pub value: Option<serde_json::Value>,
    #[serde(flatten)]
    pub visibility: Visibility,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SetTypeRequestResponse {
    success: bool,
}

pub async fn set_type(
    HandlerContext(builder): HandlerContext,
    AccessBuilder(request_ctx): AccessBuilder,
    Json(request): Json<SetTypeRequest>,
) -> ComponentResult<Json<SetTypeRequestResponse>> {
    let ctx = builder.build(request_ctx.build(request.visibility)).await?;

    let component = Component::get_by_id(&ctx, &request.component_id)
        .await?
        .ok_or(ComponentError::ComponentNotFound)?;

    // If no type was found, default to a standard "component".
    let component_type: ComponentType = match request.value {
        Some(value) => serde_json::from_value(value)?,
        None => ComponentType::Component,
    };
    component.set_type(&ctx, component_type).await?;

    WsEvent::change_set_written(&ctx)
        .await?
        .publish(&ctx)
        .await?;

    ctx.commit().await?;

    Ok(Json(SetTypeRequestResponse { success: true }))
}
