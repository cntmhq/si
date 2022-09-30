use crate::{
    impl_standard_model, pk, standard_model, standard_model_accessor, DalContext, FuncId,
    HistoryEventError, PropKind, StandardModel, StandardModelError, Timestamp, Visibility,
    WriteTenancy,
};
use postgres_types::{FromSql, ToSql};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use strum_macros::{AsRefStr, Display, EnumIter, EnumString};
use telemetry::prelude::*;
use thiserror::Error;

const LIST_FOR_FUNC: &str = include_str!("../queries/func_argument_list_for_func.sql");
const FIND_BY_NAME_FOR_FUNC: &str =
    include_str!("../queries/func_argument_find_by_name_for_func.sql");

#[derive(Debug, Error)]
pub enum FuncArgumentError {
    #[error("error serializing/deserializing json: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("pg error: {0}")]
    Pg(#[from] si_data::PgError),
    #[error("history event error: {0}")]
    HistoryEvent(#[from] HistoryEventError),
    #[error("standard model error: {0}")]
    StandardModelError(#[from] StandardModelError),
}

type FuncArgumentResult<T> = Result<T, FuncArgumentError>;

#[derive(
    Deserialize,
    Serialize,
    AsRefStr,
    Display,
    EnumIter,
    EnumString,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    ToSql,
    FromSql,
)]
pub enum FuncArgumentKind {
    Array,
    Boolean,
    Integer,
    Object,
    String,
    Map,
    Any,
}

impl From<PropKind> for FuncArgumentKind {
    fn from(prop_kind: PropKind) -> Self {
        match prop_kind {
            PropKind::Array => FuncArgumentKind::Array,
            PropKind::Boolean => FuncArgumentKind::Boolean,
            PropKind::Integer => FuncArgumentKind::Integer,
            PropKind::Object => FuncArgumentKind::Object,
            PropKind::String => FuncArgumentKind::String,
            PropKind::Map => FuncArgumentKind::Map,
        }
    }
}

pk!(FuncArgumentPk);
pk!(FuncArgumentId);

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct FuncArgument {
    pk: FuncArgumentPk,
    id: FuncArgumentId,
    func_id: FuncId,
    name: String,
    kind: FuncArgumentKind,
    element_kind: Option<FuncArgumentKind>,
    shape: Option<JsonValue>,
    #[serde(flatten)]
    tenancy: WriteTenancy,
    #[serde(flatten)]
    timestamp: Timestamp,
    #[serde(flatten)]
    visibility: Visibility,
}

impl_standard_model! {
    model: FuncArgument,
    pk: FuncArgumentPk,
    id: FuncArgumentId,
    table_name: "func_arguments",
    history_event_label_base: "func_argument",
    history_event_message_name: "Func Argument"
}

impl FuncArgument {
    pub async fn new(
        ctx: &DalContext,
        name: impl AsRef<str>,
        kind: FuncArgumentKind,
        element_kind: Option<FuncArgumentKind>,
        func_id: FuncId,
    ) -> FuncArgumentResult<Self> {
        let name = name.as_ref();
        let row = ctx
            .txns()
            .pg()
            .query_one(
                "SELECT object FROM func_argument_create_v1($1, $2, $3, $4, $5, $6)",
                &[
                    ctx.write_tenancy(),
                    ctx.visibility(),
                    &func_id,
                    &name,
                    &kind.as_ref(),
                    &element_kind.as_ref().map(|ek| ek.as_ref()),
                ],
            )
            .await?;

        Ok(standard_model::finish_create_from_row(ctx, row).await?)
    }

    standard_model_accessor!(func_id, Pk(FuncId), FuncArgumentResult);
    standard_model_accessor!(name, String, FuncArgumentResult);
    standard_model_accessor!(kind, Enum(FuncArgumentKind), FuncArgumentResult);
    standard_model_accessor!(
        element_kind,
        Option<Enum(FuncArgumentKind)>,
        FuncArgumentResult
    );
    standard_model_accessor!(shape, OptionJson<JsonValue>, FuncArgumentResult);

    /// List all [`FuncArgument`](Self) for the provided [`FuncId`](crate::FuncId).
    pub async fn list_for_func(ctx: &DalContext, func_id: FuncId) -> FuncArgumentResult<Vec<Self>> {
        let rows = ctx
            .txns()
            .pg()
            .query(
                LIST_FOR_FUNC,
                &[ctx.read_tenancy(), ctx.visibility(), &func_id],
            )
            .await?;

        Ok(standard_model::objects_from_rows(rows)?)
    }

    pub async fn find_by_name_for_func(
        ctx: &DalContext,
        name: &str,
        func_id: FuncId,
    ) -> FuncArgumentResult<Option<Self>> {
        Ok(
            match ctx
                .txns()
                .pg()
                .query_opt(
                    FIND_BY_NAME_FOR_FUNC,
                    &[ctx.read_tenancy(), ctx.visibility(), &name, &func_id],
                )
                .await?
            {
                Some(row) => standard_model::object_from_row(row)?,
                None => None,
            },
        )
    }
}
