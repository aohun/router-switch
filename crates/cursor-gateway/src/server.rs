use std::net::SocketAddr;
use std::sync::Arc;

use domain::CursorSettings;
use parking_lot::RwLock;
use tokio::net::TcpListener;

use crate::{handlers::build_router, proxy::CursorProxy, sessions::CursorSessionRegistry, Result};

pub async fn start_backend_server(
    registry: CursorSessionRegistry,
    _settings: Arc<RwLock<Option<CursorSettings>>>,
    requested_port: u16,
) -> Result<(SocketAddr, tokio::task::JoinHandle<()>)> {
    let proxy = CursorProxy::default_upstream();
    let app = build_router(registry, proxy);

    let requested = SocketAddr::from(([127, 0, 0, 1], requested_port));
    let listener = match TcpListener::bind(requested).await {
        Ok(l) => l,
        Err(e) if requested_port != 0 => {
            tracing::warn!(%requested, %e, "backend port busy; using random port");
            TcpListener::bind("127.0.0.1:0").await?
        }
        Err(e) => return Err(e.into()),
    };

    let addr = listener.local_addr()?;
    let handle = tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, app).await {
            tracing::error!(%err, "Cursor Axum backend server error");
        }
    });

    Ok((addr, handle))
}
