use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use bytes::Bytes;
use domain::{CursorKind, CursorSettings};
use parking_lot::RwLock;
use tokio::sync::{mpsc, Mutex, Notify};
use tokio_util::sync::CancellationToken;

use crate::{
    actor::{self, CursorCommand},
    prompting::PromptCompiler,
    proto::agent::v1 as pb,
    provider::{create_provider, SharedProvider},
    GatewayError, Result,
};

#[derive(Clone)]
pub struct CursorSessionHandle {
    request_id: String,
    commands: mpsc::Sender<CursorCommand>,
    output: Arc<OutputHub>,
    cancellation: CancellationToken,
    parent: Arc<OnceLock<CursorParent>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CursorParent {
    pub request_id: String,
    pub tool_call_id: String,
}

impl CursorSessionHandle {
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    pub fn subscribe(&self) -> mpsc::UnboundedReceiver<Bytes> {
        self.output.subscribe()
    }

    pub async fn command(&self, command: CursorCommand) -> Result<()> {
        self.commands
            .send(command)
            .await
            .map_err(|_| GatewayError::RunNotFound(self.request_id.clone()))
    }

    pub fn emit_frame(&self, frame: Bytes) {
        self.output.emit(frame);
    }

    pub fn emit(&self, message: &pb::AgentServerMessage) -> Result<()> {
        self.emit_frame(crate::connect::encode_message(message)?);
        Ok(())
    }

    pub fn cancel(&self) {
        self.cancellation.cancel();
    }

    pub fn close_output(&self) {
        self.output.close();
    }

    pub fn cancellation(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    pub fn set_parent(&self, parent: CursorParent) -> Result<()> {
        if parent.request_id.is_empty() || parent.tool_call_id.is_empty() {
            return Err(GatewayError::Protocol(
                "Cursor parent request and tool call ids are required".into(),
            ));
        }
        if self.parent.get().is_some_and(|current| current != &parent) {
            return Err(GatewayError::Protocol(format!(
                "conflicting parent ids for request {}",
                self.request_id
            )));
        }
        let _ = self.parent.set(parent);
        Ok(())
    }

    pub fn parent(&self) -> Option<&CursorParent> {
        self.parent.get()
    }
}

#[derive(Default)]
struct OutputHub {
    state: parking_lot::Mutex<OutputState>,
    closed: Notify,
}

#[derive(Default)]
struct OutputState {
    history: Vec<Bytes>,
    subscribers: Vec<mpsc::UnboundedSender<Bytes>>,
    closed: bool,
}

impl OutputHub {
    fn emit(&self, frame: Bytes) {
        let mut state = self.state.lock();
        if state.closed {
            return;
        }
        state.history.push(frame.clone());
        state
            .subscribers
            .retain(|subscriber| subscriber.send(frame.clone()).is_ok());
    }

    fn subscribe(&self) -> mpsc::UnboundedReceiver<Bytes> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let mut state = self.state.lock();
        for frame in &state.history {
            let _ = sender.send(frame.clone());
        }
        if !state.closed {
            state.subscribers.push(sender);
        }
        receiver
    }

    fn close(&self) {
        let mut state = self.state.lock();
        state.closed = true;
        state.subscribers.clear();
        drop(state);
        self.closed.notify_waiters();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorRoute {
    Local,
    Upstream(u64),
}

#[derive(Clone)]
pub struct CursorSessionRegistry {
    inner: Arc<RegistryInner>,
}

struct RegistryInner {
    runs: Mutex<HashMap<String, CursorSessionHandle>>,
    upstream_runs: Mutex<HashMap<String, u64>>,
    route_changed: Notify,
    settings: Arc<RwLock<Option<CursorSettings>>>,
    compiler: PromptCompiler,
    provider_override: RwLock<Option<SharedProvider>>,
}

impl CursorSessionRegistry {
    pub fn new(settings: Arc<RwLock<Option<CursorSettings>>>) -> Result<Self> {
        Ok(Self {
            inner: Arc::new(RegistryInner {
                runs: Mutex::new(HashMap::new()),
                upstream_runs: Mutex::new(HashMap::new()),
                route_changed: Notify::new(),
                settings,
                compiler: PromptCompiler::embedded()?,
                provider_override: RwLock::new(None),
            }),
        })
    }

    pub fn settings(&self) -> Arc<RwLock<Option<CursorSettings>>> {
        self.inner.settings.clone()
    }

    pub fn compiler(&self) -> &PromptCompiler {
        &self.inner.compiler
    }

    /// Test seam: inject a fake provider for local Agent runs.
    pub fn set_provider_override(&self, provider: Option<SharedProvider>) {
        *self.inner.provider_override.write() = provider;
    }

    pub fn current_kind(&self) -> CursorKind {
        self.inner
            .settings
            .read()
            .as_ref()
            .map(|s| s.kind)
            .unwrap_or(CursorKind::Official)
    }

    pub fn is_local_kind(&self) -> bool {
        matches!(self.current_kind(), CursorKind::ThirdParty)
    }

    pub fn resolve_provider(&self, model_id: &str) -> Result<SharedProvider> {
        if let Some(provider) = self.inner.provider_override.read().clone() {
            return Ok(provider);
        }
        let settings = self
            .inner
            .settings
            .read()
            .clone()
            .ok_or_else(|| GatewayError::Config("no Cursor 服务商 selected".into()))?;
        if settings.kind.is_official() {
            return Err(GatewayError::Protocol(
                "official Cursor 服务商 must proxy Agent traffic upstream".into(),
            ));
        }
        let model = if model_id.is_empty() {
            settings.model.clone()
        } else {
            model_id.to_string()
        };
        Ok(create_provider(
            &settings.provider_type,
            settings.base_url.clone(),
            settings.api_key.clone(),
            model,
            Vec::new(),
        ))
    }

    pub async fn get_or_create(&self, request_id: &str) -> Result<CursorSessionHandle> {
        if let Some(handle) = self.inner.runs.lock().await.get(request_id).cloned() {
            return Ok(handle);
        }
        let (commands, receiver) = mpsc::channel(128);
        let output = Arc::new(OutputHub::default());
        let cancellation = CancellationToken::new();
        let handle = CursorSessionHandle {
            request_id: request_id.to_string(),
            commands,
            output: output.clone(),
            cancellation: cancellation.clone(),
            parent: Arc::new(OnceLock::new()),
        };
        actor::spawn(handle.clone(), receiver, self.clone());
        self.inner
            .runs
            .lock()
            .await
            .insert(request_id.to_string(), handle.clone());
        self.inner.route_changed.notify_waiters();
        Ok(handle)
    }

    pub async fn local(&self, request_id: &str) -> Option<CursorSessionHandle> {
        self.inner.runs.lock().await.get(request_id).cloned()
    }

    pub async fn mark_upstream(&self, request_id: &str) -> u64 {
        let mut guard = self.inner.upstream_runs.lock().await;
        let generation = guard.get(request_id).copied().unwrap_or(0) + 1;
        guard.insert(request_id.to_string(), generation);
        self.inner.route_changed.notify_waiters();
        generation
    }

    pub async fn upstream(&self, request_id: &str) -> bool {
        self.inner
            .upstream_runs
            .lock()
            .await
            .contains_key(request_id)
    }

    pub async fn finish_upstream(&self, request_id: String, generation: u64) {
        let mut guard = self.inner.upstream_runs.lock().await;
        if guard.get(&request_id).copied() == Some(generation) {
            guard.remove(&request_id);
        }
    }

    pub async fn wait_route(&self, request_id: &str) -> CursorRoute {
        loop {
            if self.inner.runs.lock().await.contains_key(request_id) {
                return CursorRoute::Local;
            }
            if let Some(generation) = self
                .inner
                .upstream_runs
                .lock()
                .await
                .get(request_id)
                .copied()
            {
                return CursorRoute::Upstream(generation);
            }
            let notified = self.inner.route_changed.notified();
            if self.inner.runs.lock().await.contains_key(request_id) {
                return CursorRoute::Local;
            }
            if let Some(generation) = self
                .inner
                .upstream_runs
                .lock()
                .await
                .get(request_id)
                .copied()
            {
                return CursorRoute::Upstream(generation);
            }
            notified.await;
        }
    }

    pub async fn remove(&self, request_id: &str) {
        self.inner.runs.lock().await.remove(request_id);
    }
}
