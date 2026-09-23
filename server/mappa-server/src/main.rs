use mappa_storage::Storage;
use std::{env, net::SocketAddr};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mappa_server=info".into()),
        )
        .init();
    let database_url = env::var("DATABASE_URL")?;
    let bind_addr: SocketAddr = env::var("BIND_ADDR")?.parse()?;
    let storage = Storage::connect(&database_url).await?;
    storage.migrate().await?;
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    tracing::info!(address = %listener.local_addr()?, "listening");
    axum::serve(listener, mappa_server::router(storage)).await?;
    Ok(())
}
