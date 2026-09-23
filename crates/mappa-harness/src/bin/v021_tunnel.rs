use mappa_client_runtime::ClientRuntime;
use mappa_client_transport::{Endpoint, HttpTransport, Transport, TransportError};
use mappa_device::{ActorStore, DeviceError, LocationFix, LocationProvider, LocationStatus};
use mappa_domain::Coordinate;
use mappa_protocol::{Frame, Message, decode};
use mappa_spatial::coordinate_to_cell;
use std::{
    env,
    error::Error,
    sync::{Arc, Mutex},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tokio_postgres::NoTls;
use uuid::Uuid;

#[derive(Default)]
struct MemoryStore(Option<Vec<u8>>);
impl ActorStore for MemoryStore {
    fn load(&mut self) -> Result<Option<Vec<u8>>, DeviceError> {
        Ok(self.0.clone())
    }
    fn save(&mut self, bytes: &[u8; 16]) -> Result<(), DeviceError> {
        self.0 = Some(bytes.to_vec());
        Ok(())
    }
}

struct FakeLocation(LocationFix);
impl LocationProvider for FakeLocation {
    async fn request_location(&mut self) -> LocationStatus {
        LocationStatus::Ready(self.0)
    }
}

type TransportRecord = (Endpoint, Vec<u8>, usize);

struct CountingTransport {
    inner: HttpTransport,
    records: Arc<Mutex<Vec<TransportRecord>>>,
}
impl Transport for CountingTransport {
    async fn send(&self, endpoint: Endpoint, bytes: Vec<u8>) -> Result<Vec<u8>, TransportError> {
        let response = self.inner.send(endpoint, bytes.clone()).await?;
        if let Ok(mut records) = self.records.lock() {
            records.push((endpoint, bytes, response.len()));
        }
        Ok(response)
    }
}

fn now_ms() -> Result<i64, Box<dyn Error>> {
    Ok(i64::try_from(
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
    )?)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let url = env::var("MAPPA_API_BASE_URL")?;
    let database_url = env::var("DATABASE_URL")?;
    let http = HttpTransport::new(&url)?;
    http.check_health().await?;

    let coordinate = Coordinate::new(375_665_000, 1_269_780_000)?; // Harness fixture only.
    let cell = coordinate_to_cell(coordinate)?;
    let cell_i64 = i64::try_from(cell.0)?;
    let (db, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
    tokio::spawn(async move {
        let _ = connection.await;
    });
    let before_count: i64 = db
        .query_one("SELECT count(*) FROM posts WHERE cell_id=$1", &[&cell_i64])
        .await?
        .get(0);
    let before_revision: i64 = db
        .query_opt(
            "SELECT revision FROM cell_state WHERE cell_id=$1",
            &[&cell_i64],
        )
        .await?
        .map_or(0, |row| row.get(0));

    let captured_at_ms = now_ms()?;
    let records = Arc::new(Mutex::new(Vec::new()));
    let mut store = MemoryStore::default();
    let mut runtime = ClientRuntime::new(
        &mut store,
        FakeLocation(LocationFix {
            coordinate,
            horizontal_accuracy_m: 15.0,
            captured_at_ms,
        }),
        CountingTransport {
            inner: http,
            records: records.clone(),
        },
    )?;
    let body = format!("v0.2.1 tunnel {}", Uuid::new_v4());
    let create_started = Instant::now();
    let created = runtime.post(&body, captured_at_ms).await?;
    let create_ms = create_started.elapsed().as_millis();

    let create_bytes = {
        let sent = records.lock().map_err(|_| "record lock poisoned")?;
        sent.first().ok_or("missing create frame")?.1.clone()
    };
    let replay_started = Instant::now();
    let replay_bytes = HttpTransport::new(&url)?
        .send(Endpoint::Create, create_bytes.clone())
        .await?;
    let replay_ms = replay_started.elapsed().as_millis();
    let Frame {
        message: Message::CreatePostV2Response(replayed),
        ..
    } = decode(&replay_bytes)?
    else {
        return Err("unexpected replay response".into());
    };
    assert_eq!(replayed, created);

    let idle_at = u64::try_from(now_ms()?)?;
    runtime.camera_idle(coordinate, 5_000, idle_at);
    let query_started = Instant::now();
    assert!(
        runtime
            .refresh_visible_cells(idle_at.saturating_add(400))
            .await?
    );
    let query_ms = query_started.elapsed().as_millis();
    let cached = runtime
        .core()
        .cache()
        .get(&cell)
        .ok_or("cell query missing")?;
    assert!(
        cached
            .posts
            .iter()
            .any(|post| post.id == created.post_id && post.body == body)
    );

    let row = db
        .query_one(
            "SELECT actor_id, client_post_id, body, cell_id FROM posts WHERE id=$1",
            &[&created.post_id.0],
        )
        .await?;
    let stored_actor: Uuid = row.get(0);
    let stored_client_post_id: Option<Uuid> = row.get(1);
    let stored_body: String = row.get(2);
    let stored_cell: i64 = row.get(3);
    assert_eq!(stored_actor, runtime.actor_id().0);
    assert!(stored_client_post_id.is_some());
    assert_eq!(stored_body, body);
    assert_eq!(stored_cell, cell_i64);
    let after_count: i64 = db
        .query_one("SELECT count(*) FROM posts WHERE cell_id=$1", &[&cell_i64])
        .await?
        .get(0);
    let after_revision: i64 = db
        .query_one(
            "SELECT revision FROM cell_state WHERE cell_id=$1",
            &[&cell_i64],
        )
        .await?
        .get(0);
    assert_eq!(after_count, before_count + 1);
    assert_eq!(after_revision, before_revision + 1);
    let records = records.lock().map_err(|_| "record lock poisoned")?;
    let query_record = records
        .iter()
        .find(|(endpoint, _, _)| *endpoint == Endpoint::Query)
        .ok_or("missing query frame")?;
    println!(
        "V021 TUNNEL PASS fake_location=true tls=true before_count={before_count} after_count={after_count} before_revision={before_revision} after_revision={after_revision} create_ms={create_ms} replay_ms={replay_ms} query_ms={query_ms} create_request_bytes={} replay_response_bytes={} query_request_bytes={} query_response_bytes={}",
        create_bytes.len(),
        replay_bytes.len(),
        query_record.1.len(),
        query_record.2
    );
    Ok(())
}
