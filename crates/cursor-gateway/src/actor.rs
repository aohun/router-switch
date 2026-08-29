use tokio::sync::mpsc;

use crate::{
    agent::AgentRun,
    inbox::OrderedInbox,
    lifecycle,
    proto::agent::v1 as pb,
    sessions::{CursorSessionHandle, CursorSessionRegistry},
};

pub enum CursorCommand {
    Append {
        seqno: i64,
        message: Box<pb::AgentClientMessage>,
    },
    Abort,
    Finished,
}

pub fn spawn(
    handle: CursorSessionHandle,
    mut receiver: mpsc::Receiver<CursorCommand>,
    registry: CursorSessionRegistry,
) {
    tokio::spawn(async move {
        let mut inbox = OrderedInbox::starting_at(0);
        let mut run_tx: Option<mpsc::UnboundedSender<pb::AgentClientMessage>> = None;
        let mut started = false;

        loop {
            let command = match receiver.recv().await {
                Some(command) => command,
                None => {
                    handle.cancel();
                    break;
                }
            };
            match command {
                CursorCommand::Abort => {
                    handle.cancel();
                }
                CursorCommand::Finished => {
                    break;
                }
                CursorCommand::Append { seqno, message } => {
                    for (_seqno, message) in inbox.push(seqno, *message) {
                        match message.message {
                            Some(pb::agent_client_message::Message::RunRequest(request)) => {
                                if started {
                                    let error = crate::GatewayError::Protocol(format!(
                                        "duplicate RunRequest for request_id: {}",
                                        handle.request_id()
                                    ));
                                    let _ = lifecycle::fail(&handle, &error);
                                    break;
                                }
                                started = true;
                                let (tx, rx) = mpsc::unbounded_channel();
                                run_tx = Some(tx);
                                let handle = handle.clone();
                                let registry = registry.clone();
                                tokio::spawn(async move {
                                    let result =
                                        AgentRun::start(handle.clone(), registry, request, rx)
                                            .await;
                                    if let Err(error) = result {
                                        tracing::error!(
                                            request_id = handle.request_id(),
                                            %error,
                                            "Cursor Agent run failed"
                                        );
                                        handle.cancel();
                                        let _ = lifecycle::fail(&handle, &error);
                                    }
                                    let _ = handle.command(CursorCommand::Finished).await;
                                });
                            }
                            Some(pb::agent_client_message::Message::ClientHeartbeat(_)) => {
                                let _ = handle.emit(&crate::interaction::heartbeat());
                            }
                            Some(_) => {
                                if let Some(tx) = &run_tx {
                                    let _ = tx.send(message);
                                } else {
                                    tracing::warn!(
                                        request_id = handle.request_id(),
                                        "dropping client message before RunRequest"
                                    );
                                }
                            }
                            None => {}
                        }
                    }
                }
            }
        }
        registry.remove(handle.request_id()).await;
    });
}
