use std::{net::SocketAddr, sync::Arc};

use domain::CursorSettings;
use parking_lot::RwLock;

use crate::{
    harness::{
        account::ensure_local_ultra_account,
        ca::{CaManager, CaState},
        proxy::ProxyRuntime,
        settings::{clear_proxy_settings, write_proxy_settings},
    },
    server::start_backend_server,
    sessions::CursorSessionRegistry,
    Result,
};

pub struct CursorGatewayRuntime {
    registry: CursorSessionRegistry,
    settings: Arc<RwLock<Option<CursorSettings>>>,
    ca_manager: CaManager,
    proxy_runtime: ProxyRuntime,
    backend_addr: Option<SocketAddr>,
    backend_task: Option<tokio::task::JoinHandle<()>>,
}

impl CursorGatewayRuntime {
    pub fn new() -> Result<Self> {
        let settings = Arc::new(RwLock::new(None));
        let ca_manager = CaManager::managed()?;
        let registry = CursorSessionRegistry::new(settings.clone())?;
        Ok(Self {
            registry,
            settings,
            ca_manager,
            proxy_runtime: ProxyRuntime::default(),
            backend_addr: None,
            backend_task: None,
        })
    }

    pub fn registry(&self) -> &CursorSessionRegistry {
        &self.registry
    }

    pub fn is_running(&self) -> bool {
        self.proxy_runtime.running() && self.backend_task.as_ref().is_some_and(|t| !t.is_finished())
    }

    pub fn proxy_port(&self) -> Option<u16> {
        self.proxy_runtime.port()
    }

    pub fn backend_port(&self) -> Option<u16> {
        self.backend_addr.map(|a| a.port())
    }

    pub fn ca_state(&self) -> Result<CaState> {
        self.ca_manager.state()
    }

    pub fn ca_install_command(&self) -> Option<String> {
        self.ca_manager.install_command()
    }

    pub fn set_settings(&self, settings: Option<CursorSettings>) {
        *self.settings.write() = settings;
    }

    pub async fn start(
        &mut self,
        proxy_port: Option<u16>,
        backend_port: Option<u16>,
    ) -> Result<(u16, u16)> {
        if self.is_running() {
            return Ok((
                self.proxy_port().unwrap_or_default(),
                self.backend_port().unwrap_or_default(),
            ));
        }

        self.ca_manager.initialize_local()?;
        let loaded_ca = self.ca_manager.load()?;

        let (backend_addr, backend_handle) = start_backend_server(
            self.registry.clone(),
            self.settings.clone(),
            backend_port.unwrap_or(0),
        )
        .await?;
        self.backend_addr = Some(backend_addr);
        self.backend_task = Some(backend_handle);

        let requested_proxy_port = proxy_port.unwrap_or(2080);
        let (proxy_url, actual_proxy_port) = self
            .proxy_runtime
            .start(backend_addr, loaded_ca, requested_proxy_port)
            .await?;

        if let Err(err) = write_proxy_settings(&proxy_url) {
            tracing::warn!(%err, "could not write Cursor proxy settings to settings.json");
        }

        if let Err(err) = ensure_local_ultra_account() {
            tracing::warn!(%err, "could not ensure Cursor Ultra account in state.vscdb");
        }

        tracing::info!(
            proxy_url = %proxy_url,
            backend = %backend_addr,
            "Cursor local gateway and MITM proxy started successfully"
        );

        Ok((actual_proxy_port, backend_addr.port()))
    }

    pub async fn stop(&mut self) -> Result<()> {
        let _ = clear_proxy_settings();
        self.proxy_runtime.stop().await;
        if let Some(task) = self.backend_task.take() {
            task.abort();
        }
        self.backend_addr = None;
        tracing::info!("Cursor local gateway stopped");
        Ok(())
    }
}
