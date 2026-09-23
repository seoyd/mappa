use mappa_protocol::{CONTENT_TYPE, HEADER_BYTES, MAX_PAYLOAD_BYTES};
use reqwest::{Client, StatusCode, Url};
use std::time::Duration;
use thiserror::Error;

pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
pub const MAX_RESPONSE_BYTES: usize = HEADER_BYTES + MAX_PAYLOAD_BYTES;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Endpoint {
    Create,
    Query,
}

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("invalid Mappa server URL or insecure non-loopback URL")]
    InvalidBaseUrl,
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("unexpected HTTP status {0}")]
    Status(u16),
    #[error("unexpected response content type")]
    ContentType,
    #[error("response body exceeds limit")]
    TooLarge,
}

#[allow(async_fn_in_trait)] // Internal runtime contract, including single-threaded UI callers.
pub trait Transport {
    async fn send(&self, endpoint: Endpoint, bytes: Vec<u8>) -> Result<Vec<u8>, TransportError>;
}

pub struct HttpTransport {
    client: Client,
    base: Url,
}

impl HttpTransport {
    pub fn new(base_url: &str) -> Result<Self, TransportError> {
        let base = Url::parse(base_url).map_err(|_| TransportError::InvalidBaseUrl)?;
        let loopback = matches!(base.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if !matches!(base.scheme(), "https" | "http")
            || (base.scheme() == "http" && !loopback)
            || base.path() != "/"
            || base.query().is_some()
            || base.fragment().is_some()
            || !base.username().is_empty()
            || base.password().is_some()
        {
            return Err(TransportError::InvalidBaseUrl);
        }
        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()?;
        Ok(Self { client, base })
    }

    pub async fn check_health(&self) -> Result<(), TransportError> {
        let url = self
            .base
            .join("health")
            .map_err(|_| TransportError::InvalidBaseUrl)?;
        let response = self.client.get(url).send().await?;
        if response.status() != StatusCode::OK {
            return Err(TransportError::Status(response.status().as_u16()));
        }
        Ok(())
    }
}

impl Transport for HttpTransport {
    async fn send(&self, endpoint: Endpoint, bytes: Vec<u8>) -> Result<Vec<u8>, TransportError> {
        let (path, expected) = match endpoint {
            Endpoint::Create => ("v1/posts", StatusCode::CREATED),
            Endpoint::Query => ("v1/cells/query", StatusCode::OK),
        };
        let url = self
            .base
            .join(path)
            .map_err(|_| TransportError::InvalidBaseUrl)?;
        let mut response = self
            .client
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, CONTENT_TYPE)
            .body(bytes)
            .send()
            .await?;
        let status = response.status();
        if status != expected {
            return Err(TransportError::Status(status.as_u16()));
        }
        if response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            != Some(CONTENT_TYPE)
        {
            return Err(TransportError::ContentType);
        }
        if response
            .content_length()
            .is_some_and(|len| len > MAX_RESPONSE_BYTES as u64)
        {
            return Err(TransportError::TooLarge);
        }
        let mut out = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if out
                .len()
                .checked_add(chunk.len())
                .is_none_or(|len| len > MAX_RESPONSE_BYTES)
            {
                return Err(TransportError::TooLarge);
            }
            out.extend_from_slice(&chunk);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn fake_server(reply: Vec<u8>) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request).await;
            let _ = stream.write_all(&reply).await;
        });
        url
    }

    #[test]
    fn only_https_or_loopback_http() {
        assert!(HttpTransport::new("http://127.0.0.1:3000/").is_ok());
        assert!(HttpTransport::new("https://mappa.example/").is_ok());
        assert!(HttpTransport::new("http://mappa.example/").is_err());
        assert!(HttpTransport::new("https://mappa.example/path").is_err());
    }

    #[tokio::test]
    async fn health_and_error_responses_are_typed() {
        let health = fake_server(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n".to_vec()).await;
        HttpTransport::new(&health)
            .unwrap()
            .check_health()
            .await
            .unwrap();

        let wrong_type = fake_server(
            b"HTTP/1.1 201 Created\r\nContent-Type: text/plain\r\nContent-Length: 0\r\n\r\n"
                .to_vec(),
        )
        .await;
        assert!(matches!(
            HttpTransport::new(&wrong_type)
                .unwrap()
                .send(Endpoint::Create, vec![1])
                .await,
            Err(TransportError::ContentType)
        ));

        let oversized = fake_server(format!("HTTP/1.1 201 Created\r\nContent-Type: {CONTENT_TYPE}\r\nContent-Length: {}\r\n\r\n", MAX_RESPONSE_BYTES + 1).into_bytes()).await;
        assert!(matches!(
            HttpTransport::new(&oversized)
                .unwrap()
                .send(Endpoint::Create, vec![1])
                .await,
            Err(TransportError::TooLarge)
        ));

        for status in [409, 500] {
            let url = fake_server(
                format!("HTTP/1.1 {status} Error\r\nContent-Length: 0\r\n\r\n").into_bytes(),
            )
            .await;
            assert!(
                matches!(HttpTransport::new(&url).unwrap().send(Endpoint::Create, vec![1]).await, Err(TransportError::Status(code)) if code == status)
            );
        }
    }

    #[tokio::test]
    async fn timeout_and_unavailable_server_are_errors() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let mut transport = HttpTransport::new(&url).unwrap();
        transport.client = Client::builder()
            .timeout(Duration::from_millis(40))
            .build()
            .unwrap();
        let server = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
        });
        assert!(
            matches!(transport.send(Endpoint::Create, vec![1]).await, Err(TransportError::Http(error)) if error.is_timeout())
        );
        server.await.unwrap();
        assert!(matches!(
            transport.send(Endpoint::Create, vec![1]).await,
            Err(TransportError::Http(_))
        ));
    }
}
