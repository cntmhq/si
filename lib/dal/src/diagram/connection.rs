use serde::{Deserialize, Serialize};

use crate::edge::{Edge, EdgeId, EdgeKind};

use crate::change_status::ChangeStatus;
use crate::diagram::DiagramResult;
use crate::socket::SocketId;
use crate::{node::NodeId, DalContext, DiagramError, StandardModel};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Vertex {
    pub node_id: NodeId,
    pub socket_id: SocketId,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub id: EdgeId,
    pub classification: EdgeKind,
    pub source: Vertex,
    pub destination: Vertex,
}

impl Connection {
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        ctx: &DalContext,
        from_node_id: NodeId,
        from_socket_id: SocketId,
        to_node_id: NodeId,
        to_socket_id: SocketId,
        edge_kind: EdgeKind,
    ) -> DiagramResult<Self> {
        let edge = Edge::new_for_connection(
            ctx,
            to_node_id,
            to_socket_id,
            from_node_id,
            from_socket_id,
            edge_kind,
        )
        .await?;
        Ok(Connection::from_edge(&edge))
    }

    pub async fn list(ctx: &DalContext) -> DiagramResult<Vec<Self>> {
        let edges = Edge::list(ctx).await?;
        let connections = edges.iter().map(Self::from_edge).collect::<Vec<Self>>();
        Ok(connections)
    }

    pub fn from_edge(edge: &Edge) -> Self {
        Self {
            id: *edge.id(),
            classification: edge.kind().clone(),
            source: Vertex {
                node_id: edge.tail_node_id(),
                socket_id: edge.tail_socket_id(),
            },
            destination: Vertex {
                node_id: edge.head_node_id(),
                socket_id: edge.head_socket_id(),
            },
        }
    }

    pub fn source(&self) -> (NodeId, SocketId) {
        (self.source.node_id, self.source.socket_id)
    }

    pub fn destination(&self) -> (NodeId, SocketId) {
        (self.destination.node_id, self.destination.socket_id)
    }

    pub async fn delete_for_edge(ctx: &DalContext, edge_id: EdgeId) -> DiagramResult<()> {
        let mut edge = Edge::get_by_id(ctx, &edge_id)
            .await?
            .ok_or(DiagramError::EdgeNotFound)?;
        edge.delete_and_propagate(ctx).await?;
        Ok(())
    }

    pub async fn restore_for_edge(ctx: &DalContext, edge_id: EdgeId) -> DiagramResult<()> {
        Edge::restore_by_id(ctx, edge_id).await?;
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiagramEdgeView {
    id: String,
    from_node_id: String,
    from_socket_id: String,
    to_node_id: String,
    to_socket_id: String,
    change_status: ChangeStatus,
}

impl DiagramEdgeView {
    pub fn id(&self) -> &str {
        &self.id
    }
}

impl DiagramEdgeView {
    pub fn from_with_change_status(conn: Connection, change_status: ChangeStatus) -> Self {
        Self {
            id: conn.id.to_string(),
            from_node_id: conn.source.node_id.to_string(),
            from_socket_id: conn.source.socket_id.to_string(),
            to_node_id: conn.destination.node_id.to_string(),
            to_socket_id: conn.destination.socket_id.to_string(),
            change_status,
        }
    }
}

impl From<Connection> for DiagramEdgeView {
    fn from(conn: Connection) -> Self {
        DiagramEdgeView::from_with_change_status(conn, ChangeStatus::Unmodified)
    }
}
