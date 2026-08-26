use std::net::SocketAddr;

use hudsucker::{
    certificate_authority::RcgenAuthority,
    hyper::{Request, Uri},
    rustls::crypto::aws_lc_rs,
    Body, HttpContext, HttpHandler, Proxy, RequestOrResponse,
};
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};

use crate::{harness::ca::LoadedCa, GatewayError, Result};

pub const UPSTREAM_URL_HEADER: &str = "x-server-upstream-url";

#[derive(Default)]
pub struct ProxyRuntime {
    url: Option<String>,
    port: Option<u16>,
    stop: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}

impl ProxyRuntime {
    pub fn running(&self) -> bool {
        self.task.as_ref().is_some_and(|task| !task.is_finished())
    }

    pub fn url(&self) -> Option<String> {
        self.running().then(|| self.url.clone()).flatten()
    }

    pub fn port(&self) -> Option<u16> {
        self.running().then(|| self.port).flatten()
    }

    pub async fn start(
        &mut self,
        backend: SocketAddr,
        ca: LoadedCa,
        requested_port: u16,
    ) -> Result<(String, u16)> {
        if let Some(url) = self.url() {
            return Ok((url, self.port.unwrap_or_default()));
        }
        let listener = bind_proxy_listener(requested_port).await?;
        let address = listener.local_addr()?;
        let (stop, done) = oneshot::channel();
        let authority = RcgenAuthority::new(ca.issuer, 1_000, aws_lc_rs::default_provider());
        let proxy = Proxy::builder()
            .with_listener(listener)
            .with_ca(authority)
            .with_rustls_connector(aws_lc_rs::default_provider())
            .with_http_handler(CursorRelay { backend })
            .with_graceful_shutdown(async move {
                let _ = done.await;
            })
            .build()
            .map_err(|error| GatewayError::Store(format!("build Cursor proxy: {error}")))?;
        self.stop = Some(stop);
        self.url = Some(format!("http://{address}"));
        self.port = Some(address.port());
        self.task = Some(tokio::spawn(async move {
            if let Err(error) = proxy.start().await {
                tracing::error!(%error, "Cursor MITM proxy stopped unexpectedly");
            }
        }));
        Ok((self.url.clone().unwrap(), address.port()))
    }

    pub async fn stop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(task) = self.task.take() {
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), task).await;
        }
        self.url = None;
        self.port = None;
    }
}

async fn bind_proxy_listener(requested_port: u16) -> Result<TcpListener> {
    let requested = SocketAddr::from(([127, 0, 0, 1], requested_port));
    match TcpListener::bind(requested).await {
        Ok(listener) => Ok(listener),
        Err(error) if requested_port != 0 => {
            tracing::warn!(%requested, %error, "configured proxy port unavailable; selecting a random port");
            Ok(TcpListener::bind("127.0.0.1:0").await?)
        }
        Err(error) => Err(error.into()),
    }
}

#[derive(Clone)]
struct CursorRelay {
    backend: SocketAddr,
}

impl HttpHandler for CursorRelay {
    async fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        mut request: Request<Body>,
    ) -> RequestOrResponse {
        let original = request.uri().clone();
        let is_cursor = is_cursor_host(original.host().unwrap_or_default());
        let locally_routed = is_local_path(original.path());

        if is_cursor && locally_routed {
            if let Ok(value) = original.to_string().parse() {
                request.headers_mut().insert(UPSTREAM_URL_HEADER, value);
            }
            let path = original
                .path_and_query()
                .map(|value| value.as_str())
                .unwrap_or("/");
            if let Ok(uri) = format!("http://{}{}", self.backend, path).parse::<Uri>() {
                *request.uri_mut() = uri;
            }
        }
        request.into()
    }

    async fn should_intercept_connect(
        &mut self,
        _ctx: &HttpContext,
        request: &Request<Body>,
    ) -> bool {
        request
            .uri()
            .authority()
            .is_some_and(|authority| is_cursor_host(authority.host()))
    }

    async fn should_intercept_tls(
        &mut self,
        _ctx: &HttpContext,
        hello: hudsucker::rustls::server::ClientHello<'_>,
    ) -> bool {
        hello.server_name().is_some_and(is_cursor_host)
    }
}

pub fn is_cursor_host(host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    matches!(host.as_str(), "api2.cursor.sh" | "api3.cursor.sh") || host.ends_with(".cursor.sh")
}

pub fn is_local_path(path: &str) -> bool {
    matches!(
        path,
        "/agent.v1.AgentService/RunSSE"
            | "/agent.v1.AgentService/GetUsableModels"
            | "/aiserver.v1.BidiService/BidiAppend"
            | "/aiserver.v1.AiService/AvailableModels"
            | "/aiserver.v1.AiService/GetUsableModels"
            | "/aiserver.v1.AiService/StreamChat"
            | "/aiserver.v1.AiService/StreamChat2"
            | "/aiserver.v1.AiService/StreamChatTree"
            | "/aiserver.v1.AiService/StreamChatWithTools"
            | "/aiserver.v1.AiService/StreamUnifiedChatWithTools"
            | "/aiserver.v1.AiService/StreamComposer"
            | "/aiserver.v1.AiService/StreamComposer2"
            | "/aiserver.v1.AiService/StreamComposerUnified"
            | "/aiserver.v1.AiService/CheckUser"
            | "/aiserver.v1.AiService/CheckAuth"
            | "/aiserver.v1.AiService/CheckSubscription"
            | "/aiserver.v1.AiService/GetUser"
            | "/aiserver.v1.AiService/ServerTime"
            | "/aiserver.v1.AiService/GetServerConfig"
            | "/aiserver.v1.AiService/GetDefaultModelNudgeData"
            | "/aiserver.v1.ChatService/StreamChat"
            | "/aiserver.v1.ChatService/StreamChat2"
            | "/aiserver.v1.AuthService/GetEmail"
            | "/aiserver.v1.DashboardService/GetMe"
            | "/aiserver.v1.DashboardService/GetUser"
            | "/aiserver.v1.DashboardService/GetTeams"
            | "/aiserver.v1.DashboardService/GetUserProfile"
            | "/aiserver.v1.DashboardService/GetCurrentPeriodUsage"
            | "/aiserver.v1.DashboardService/GetUsage"
            | "/aiserver.v1.DashboardService/GetUsageLimitStatusAndActiveGrants"
            | "/aiserver.v1.DashboardService/GetPlanInfo"
            | "/aiserver.v1.DashboardService/GetSubscription"
            | "/aiserver.v1.DashboardService/GetAccountDetails"
            | "/aiserver.v1.DashboardService/IsOnNewPricing"
            | "/aiserver.v1.DashboardService/GetUserPrivacyMode"
            | "/aiserver.v1.DashboardService/GetManagedSkills"
            | "/aiserver.v1.DashboardService/GetGlassEarlyPreviewEnrollment"
            | "/aiserver.v1.DashboardService/GetTokenUsage"
            | "/aiserver.v1.DashboardService/CheckUser"
            | "/aiserver.v1.AnalyticsService/BootstrapStatsig"
            | "/aiserver.v1.AnalyticsService/GetFirstWindowStatsigDecision"
            | "/auth/full_stripe_profile"
            | "/auth/stripe_profile"
            | "/auth/stripe_customer"
            | "/api/auth/stripe_customer"
            | "/auth/has_valid_payment_method"
            | "/auth/poll"
            | "/auth/session"
            | "/api/auth/session"
            | "/auth/user"
            | "/api/auth/user"
            | "/oauth/token"
            | "/auth/logout"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_interception_to_cursor_hosts_and_local_paths() {
        assert!(is_cursor_host("api2.cursor.sh"));
        assert!(is_cursor_host("api3.cursor.sh"));
        assert!(is_cursor_host("repo42.cursor.sh"));
        assert!(!is_cursor_host("example.com"));
        assert!(is_local_path("/agent.v1.AgentService/RunSSE"));
        assert!(is_local_path(
            "/aiserver.v1.AnalyticsService/BootstrapStatsig"
        ));
        assert!(is_local_path("/aiserver.v1.AuthService/GetEmail"));
        assert!(is_local_path("/aiserver.v1.DashboardService/GetMe"));
        assert!(!is_local_path("/unrelated"));
    }
}
