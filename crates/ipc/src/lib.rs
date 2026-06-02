use anyhow::{Context, Result};
use hyper_util::rt::TokioIo;
use std::path::{Path, PathBuf};
use tokio::net::UnixStream;
use tonic::transport::{Channel, Endpoint};
use tower::service_fn;

pub mod pb {
    tonic::include_proto!("network_manager.v1");
}

pub fn default_socket_path() -> PathBuf {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    std::env::temp_dir().join(format!("network-manager-{user}.sock"))
}

pub async fn connect_uds(
    path: impl AsRef<Path>,
) -> Result<pb::network_manager_client::NetworkManagerClient<Channel>> {
    let path = path.as_ref().to_path_buf();
    let endpoint = Endpoint::try_from("http://[::]:50051").context("creating tonic endpoint")?;
    let channel = endpoint
        .connect_with_connector(service_fn(move |_uri: http::Uri| {
            let path = path.clone();
            async move { UnixStream::connect(path).await.map(TokioIo::new) }
        }))
        .await
        .context("connecting to network-manager daemon socket")?;

    Ok(pb::network_manager_client::NetworkManagerClient::new(
        channel,
    ))
}

pub fn socket_parent(path: &Path) -> Option<&Path> {
    path.parent()
}
