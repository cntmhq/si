use crate::diagram::geometry::GeometryId;
use crate::diagram::{DiagramError, DiagramResult};
use crate::layer_db_types::{ViewContent, ViewContentV1};
use crate::workspace_snapshot::node_weight::category_node_weight::CategoryNodeKind;
use crate::workspace_snapshot::node_weight::traits::SiVersionedNodeWeight;
use crate::workspace_snapshot::node_weight::view_node_weight::ViewNodeWeight;
use crate::workspace_snapshot::node_weight::NodeWeight;
use crate::{
    id, implement_add_edge_to, ChangeSetId, EdgeWeightKindDiscriminants, Timestamp,
    WorkspaceSnapshotError, WsEvent, WsEventResult, WsPayload,
};
use crate::{DalContext, EdgeWeightKind};
use chrono::Utc;
use petgraph::Outgoing;
use serde::{Deserialize, Serialize};
use si_events::ulid::Ulid;
use si_events::{ComponentId, ContentHash};
use si_frontend_types::RawGeometry;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

id!(ViewId);

impl From<ViewId> for si_events::ViewId {
    fn from(value: ViewId) -> Self {
        value.into_inner().into()
    }
}

impl From<si_events::ViewId> for ViewId {
    fn from(value: si_events::ViewId) -> Self {
        Self(value.into_raw_id())
    }
}

/// Represents spatial data for something to be shown on a view
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct View {
    id: ViewId,
    name: String,
    #[serde(flatten)]
    timestamp: Timestamp,
}

impl View {
    implement_add_edge_to!(
        source_id: ViewId,
        destination_id: GeometryId,
        add_fn: add_edge_to_geometry,
        discriminant: EdgeWeightKindDiscriminants::Use,
        result: DiagramResult,
    );

    implement_add_edge_to!(
        source_id: Ulid,
        destination_id: ViewId,
        add_fn: add_category_edge,
        discriminant: EdgeWeightKindDiscriminants::Use,
        result: DiagramResult,
    );

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn id(&self) -> ViewId {
        self.id
    }

    pub fn timestamp(&self) -> &Timestamp {
        &self.timestamp
    }

    pub async fn is_default(&self, ctx: &DalContext) -> DiagramResult<bool> {
        let default_id = Self::get_id_for_default(ctx).await?;

        Ok(default_id == self.id)
    }

    fn assemble(node_weight: ViewNodeWeight, content: ViewContent) -> Self {
        let content = content.extract();

        Self {
            id: node_weight.id().into(),
            timestamp: content.timestamp,
            name: content.name,
        }
    }

    pub async fn new(ctx: &DalContext, name: impl AsRef<str>) -> DiagramResult<Self> {
        let snap = ctx.workspace_snapshot()?;
        let id = snap.generate_ulid().await?;
        let lineage_id = snap.generate_ulid().await?;

        let content = ViewContent::V1(ViewContentV1 {
            timestamp: Timestamp::now(),
            name: name.as_ref().to_owned(),
        });

        let (content_address, _) = ctx.layer_db().cas().write(
            Arc::new(content.clone().into()),
            None,
            ctx.events_tenancy(),
            ctx.events_actor(),
        )?;

        let node_weight = NodeWeight::new_view(id, lineage_id, content_address);
        snap.add_or_replace_node(node_weight.clone()).await?;

        // Root --> View Category --> View (this)
        let view_category_id = snap
            .get_category_node_or_err(None, CategoryNodeKind::View)
            .await?;
        Self::add_category_edge(ctx, view_category_id, id.into(), EdgeWeightKind::new_use())
            .await?;

        Ok(Self::assemble(node_weight.get_view_node_weight()?, content))
    }

    pub async fn get_by_id(ctx: &DalContext, view_id: ViewId) -> DiagramResult<Self> {
        let (node_weight, content) = Self::get_node_weight_and_content(ctx, view_id).await?;
        Ok(Self::assemble(node_weight, content))
    }

    pub async fn find_by_name(ctx: &DalContext, name: &str) -> DiagramResult<Option<Self>> {
        for view_node_weight in Self::list_node_weights(ctx).await? {
            let content = Self::try_get_content(ctx, &view_node_weight.content_hash())
                .await?
                .ok_or(WorkspaceSnapshotError::MissingContentFromStore(
                    view_node_weight.id(),
                ))?;

            let view = Self::assemble(view_node_weight, content);

            if view.name == name {
                return Ok(Some(view));
            }
        }

        Ok(None)
    }

    async fn list_node_weights(ctx: &DalContext) -> DiagramResult<Vec<ViewNodeWeight>> {
        let snap = ctx.workspace_snapshot()?;

        let category_node = snap
            .get_category_node(None, CategoryNodeKind::View)
            .await?
            .ok_or(DiagramError::ViewCategoryNotFound)?;

        let mut views = vec![];
        for view_idx in snap
            .outgoing_targets_for_edge_weight_kind(category_node, EdgeWeightKindDiscriminants::Use)
            .await?
        {
            let view_node_weight = snap
                .get_node_weight(view_idx)
                .await?
                .get_view_node_weight()?;

            views.push(view_node_weight);
        }

        Ok(views)
    }

    pub async fn list(ctx: &DalContext) -> DiagramResult<Vec<Self>> {
        let mut views = vec![];
        for view_node_weight in Self::list_node_weights(ctx).await? {
            let content = Self::try_get_content(ctx, &view_node_weight.content_hash())
                .await?
                .ok_or(WorkspaceSnapshotError::MissingContentFromStore(
                    view_node_weight.id(),
                ))?;

            views.push(Self::assemble(view_node_weight, content));
        }

        Ok(views)
    }

    pub async fn get_id_for_default(ctx: &DalContext) -> DiagramResult<ViewId> {
        let snap = ctx.workspace_snapshot()?;

        let view_category_id = snap
            .get_category_node(None, CategoryNodeKind::View)
            .await?
            .ok_or(DiagramError::ViewCategoryNotFound)?;

        let mut maybe_default_view = None;
        for (edge, _from_idx, to_idx) in snap
            .edges_directed_for_edge_weight_kind(
                view_category_id,
                Outgoing,
                EdgeWeightKindDiscriminants::Use,
            )
            .await?
        {
            if *edge.kind() == EdgeWeightKind::new_use_default() {
                maybe_default_view = Some(snap.get_node_weight(to_idx).await?.id())
            }
        }

        let Some(default_view) = maybe_default_view else {
            return Err(DiagramError::DefaultViewNotFound);
        };

        Ok(default_view.into())
    }

    async fn get_node_weight_and_content(
        ctx: &DalContext,
        view_id: ViewId,
    ) -> DiagramResult<(ViewNodeWeight, ViewContent)> {
        Self::try_get_node_weight_and_content(ctx, view_id)
            .await?
            .ok_or(DiagramError::ViewNotFound(view_id))
    }

    async fn try_get_node_weight_and_content(
        ctx: &DalContext,
        view_id: ViewId,
    ) -> DiagramResult<Option<(ViewNodeWeight, ViewContent)>> {
        let id: Ulid = view_id.into();

        let Some(node_index) = ctx.workspace_snapshot()?.get_node_index_by_id_opt(id).await else {
            return Ok(None);
        };

        let node_weight = ctx
            .workspace_snapshot()?
            .get_node_weight(node_index)
            .await?
            .get_view_node_weight()?;

        let hash = node_weight.content_hash();

        let content = Self::try_get_content(ctx, &hash).await?.ok_or(
            WorkspaceSnapshotError::MissingContentFromStore(view_id.into()),
        )?;

        Ok(Some((node_weight, content)))
    }

    async fn try_get_content(
        ctx: &DalContext,
        hash: &ContentHash,
    ) -> DiagramResult<Option<ViewContent>> {
        Ok(ctx.layer_db().cas().try_read_as(hash).await?)
    }

    pub async fn add_geometry_by_id(
        ctx: &DalContext,
        view_id: ViewId,
        geometry_id: GeometryId,
    ) -> DiagramResult<()> {
        Self::add_edge_to_geometry(
            ctx,
            view_id,
            geometry_id,
            EdgeWeightKind::Use { is_default: false },
        )
        .await
    }

    pub async fn set_name(&mut self, ctx: &DalContext, name: impl AsRef<str>) -> DiagramResult<()> {
        let (hash, _) = ctx.layer_db().cas().write(
            Arc::new(
                ViewContent::V1(ViewContentV1 {
                    timestamp: Timestamp {
                        created_at: self.timestamp.created_at,
                        updated_at: Utc::now(),
                    },
                    name: name.as_ref().to_owned(),
                })
                .into(),
            ),
            None,
            ctx.events_tenancy(),
            ctx.events_actor(),
        )?;

        ctx.workspace_snapshot()?
            .update_content(self.id.into(), hash)
            .await?;

        Ok(())
    }

    pub async fn set_default(ctx: &DalContext, view_id: ViewId) -> DiagramResult<()> {
        let snap = ctx.workspace_snapshot()?;

        let view_category_id = snap
            .get_category_node(None, CategoryNodeKind::View)
            .await?
            .ok_or(DiagramError::ViewCategoryNotFound)?;

        // Update edge to old default
        for (edge, from_idx, to_idx) in snap
            .edges_directed_for_edge_weight_kind(
                view_category_id,
                Outgoing,
                EdgeWeightKindDiscriminants::Use,
            )
            .await?
        {
            if *edge.kind() == EdgeWeightKind::new_use_default() {
                // We have found the existing previous default view
                // we now need to update that edge to be a Use
                snap.remove_edge(from_idx, to_idx, edge.kind().into())
                    .await?;

                Self::add_category_edge(
                    ctx,
                    view_category_id,
                    snap.get_node_weight(to_idx).await?.id().into(),
                    EdgeWeightKind::new_use(),
                )
                .await?;
            }
        }

        snap.remove_edge_for_ulids(view_category_id, view_id, EdgeWeightKindDiscriminants::Use)
            .await?;

        Self::add_category_edge(
            ctx,
            view_category_id,
            view_id,
            EdgeWeightKind::new_use_default(),
        )
        .await?;

        Ok(())
    }
}

/// Frontend representation for a [View](View).
/// Yeah, it's a silly name, but all the other frontend representation structs are *View,
/// so we either keep it or change everything.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct ViewView {
    id: ViewId,
    name: String,
    is_default: bool,
    #[serde(flatten)]
    timestamp: Timestamp,
}

impl ViewView {
    pub async fn from_view(ctx: &DalContext, view: View) -> DiagramResult<Self> {
        Ok(ViewView {
            id: view.id(),
            name: view.name().to_owned(),
            is_default: view.is_default(ctx).await?,
            timestamp: view.timestamp().to_owned(),
        })
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize, Clone, Default)]
pub struct ViewComponentsUpdateSingle {
    pub added: HashMap<ComponentId, RawGeometry>,
    pub removed: HashSet<ComponentId>,
}

pub type ViewComponentsUpdateList = HashMap<ViewId, ViewComponentsUpdateSingle>;

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize, Clone)]
pub struct ViewComponentsUpdatePayload {
    change_set_id: ChangeSetId,
    updates_by_view: ViewComponentsUpdateList,
    // Used so the client can ignore the messages it caused. created by the frontend, and not stored
    client_ulid: Option<ulid::Ulid>,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize, Clone)]
pub struct ViewWsPayload {
    change_set_id: ChangeSetId,
    view: ViewView,
    // Used so the client can ignore the messages it caused. created by the frontend, and not stored
    client_ulid: Option<ulid::Ulid>,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize, Clone)]
pub struct ViewDeletedPayload {
    change_set_id: ChangeSetId,
    view_id: ViewId,
    // Used so the client can ignore the messages it caused. created by the frontend, and not stored
    client_ulid: Option<ulid::Ulid>,
}

impl WsEvent {
    pub async fn view_created(
        ctx: &DalContext,
        view: ViewView,
        client_ulid: Option<ulid::Ulid>,
    ) -> WsEventResult<Self> {
        WsEvent::new(
            ctx,
            WsPayload::ViewCreated(ViewWsPayload {
                change_set_id: ctx.change_set_id(),
                view,
                client_ulid,
            }),
        )
        .await
    }

    pub async fn view_updated(
        ctx: &DalContext,
        view: ViewView,
        client_ulid: Option<ulid::Ulid>,
    ) -> WsEventResult<Self> {
        WsEvent::new(
            ctx,
            WsPayload::ViewUpdated(ViewWsPayload {
                change_set_id: ctx.change_set_id(),
                view,
                client_ulid,
            }),
        )
        .await
    }

    #[allow(unused)]
    pub async fn view_deleted(
        ctx: &DalContext,
        view_id: ViewId,
        client_ulid: Option<ulid::Ulid>,
    ) -> WsEventResult<Self> {
        WsEvent::new(
            ctx,
            WsPayload::ViewDeleted(ViewDeletedPayload {
                change_set_id: ctx.change_set_id(),
                view_id,
                client_ulid,
            }),
        )
        .await
    }

    pub async fn view_components_update(
        ctx: &DalContext,
        updates_by_view: ViewComponentsUpdateList,
        client_ulid: Option<ulid::Ulid>,
    ) -> WsEventResult<Self> {
        WsEvent::new(
            ctx,
            WsPayload::ViewComponentsUpdate(ViewComponentsUpdatePayload {
                change_set_id: ctx.change_set_id(),
                updates_by_view,
                client_ulid,
            }),
        )
        .await
    }
}
