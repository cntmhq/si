use std::sync::Arc;

use futures::future::BoxFuture;
use naxum::middleware::post_process;
use si_data_nats::async_nats;
use telemetry::prelude::*;

#[derive(Clone, Debug)]
pub(crate) struct DeleteMessageOnSuccess {
    stream: async_nats::jetstream::stream::Stream,
}

impl DeleteMessageOnSuccess {
    pub(crate) fn new(stream: async_nats::jetstream::stream::Stream) -> Self {
        Self { stream }
    }
}

impl post_process::OnSuccess for DeleteMessageOnSuccess {
    fn call(
        &mut self,
        head: Arc<naxum::Head>,
        info: Arc<post_process::Info>,
    ) -> BoxFuture<'static, ()> {
        let stream = self.stream.clone();

        Box::pin(async move {
            trace!("deleting message on success");
            if let Err(err) = stream.delete_message(info.stream_sequence).await {
                warn!(
                    si.error.message = ?err,
                    subject = head.subject.as_str(),
                    "failed to delete the message",
                );
            }
        })
    }
}
