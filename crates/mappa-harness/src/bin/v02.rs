use mappa_client_runtime::ClientRuntime;
use mappa_client_transport::HttpTransport;
use mappa_device::{
    ActorStore, DeviceError, LocationFix, LocationProvider, LocationStatus, degrees_to_coordinate,
    load_or_create_actor,
};
use mappa_domain::{ActorId, ClientPostId, Coordinate, PostKind};
use mappa_protocol::{
    CONTENT_TYPE, CellQuery, CellResult, CreatePostResponse, CreatePostV2Request, ErrorCode, Frame,
    Message, decode, encode,
};
use mappa_spatial::coordinate_to_cell;
use mappa_storage::Storage;
use std::{
    env,
    error::Error,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tokio::task::JoinSet;
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
struct FakeLocation(LocationStatus);
impl LocationProvider for FakeLocation {
    async fn request_location(&mut self) -> LocationStatus {
        self.0
    }
}

fn now_ms() -> Result<i64, Box<dyn Error>> {
    Ok(i64::try_from(
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
    )?)
}

async fn http_create(
    http: &reqwest::Client,
    address: std::net::SocketAddr,
    frame: &Frame,
) -> Result<(reqwest::StatusCode, Vec<u8>), Box<dyn Error>> {
    let response = http
        .post(format!("http://{address}/v1/posts"))
        .header("content-type", CONTENT_TYPE)
        .body(encode(frame)?)
        .send()
        .await?;
    let status = response.status();
    Ok((status, response.bytes().await?.to_vec()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let url = env::var("DATABASE_URL")?;
    let storage = Storage::connect(&url).await?;
    storage.migrate().await?;
    storage.migrate().await?; // migration rerun is safe
    let coordinate = Coordinate::new(375_665_000, 1_269_780_000)?;
    let cell = coordinate_to_cell(coordinate)?;
    let before = storage
        .query_cells(&[CellQuery {
            cell_id: cell,
            known_revision: 0,
        }])
        .await?;
    let CellResult::Snapshot {
        revision: before_revision,
        posts: before_posts,
        ..
    } = &before.cells[0]
    else {
        return Err("missing baseline".into());
    };
    let before_revision = *before_revision;
    let before_count = before_posts.len();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server =
        tokio::spawn(
            async move { axum::serve(listener, mappa_server::router(storage.clone())).await },
        );
    let http = reqwest::Client::new();
    let actor = ActorId(Uuid::new_v4());
    let client_post_id = ClientPostId(Uuid::new_v4());
    let request = CreatePostV2Request {
        actor_id: actor,
        client_post_id,
        coordinate,
        kind: PostKind::General,
        body: format!("v0.2 {}", Uuid::new_v4()),
    };
    let first_frame = Frame {
        request_id: 1,
        message: Message::CreatePostV2Request(request.clone()),
    };
    let sample_request = encode(&first_frame)?;
    let started = Instant::now();
    for _ in 0..10_000 {
        std::hint::black_box(encode(&first_frame)?);
    }
    let encode_10k_us = started.elapsed().as_micros();
    let started = Instant::now();
    for _ in 0..10_000 {
        std::hint::black_box(decode(&sample_request)?);
    }
    let decode_10k_us = started.elapsed().as_micros();
    let started = Instant::now();
    let (status, first_bytes) = http_create(&http, address, &first_frame).await?;
    let first_us = started.elapsed().as_micros();
    assert_eq!(status, reqwest::StatusCode::CREATED);
    let Message::CreatePostV2Response(first) = decode(&first_bytes)?.message else {
        return Err("wrong first response".into());
    };
    assert_eq!(first.cell_revision, before_revision + 1);
    let started = Instant::now();
    let (status, replay_bytes) = http_create(&http, address, &first_frame).await?;
    let replay_us = started.elapsed().as_micros();
    assert_eq!(status, reqwest::StatusCode::CREATED);
    let Message::CreatePostV2Response(replay) = decode(&replay_bytes)?.message else {
        return Err("wrong replay response".into());
    };
    assert_eq!(replay, first);
    let mut conflict = request.clone();
    conflict.body.push_str(" changed");
    let (status, conflict_bytes) = http_create(
        &http,
        address,
        &Frame {
            request_id: 2,
            message: Message::CreatePostV2Request(conflict),
        },
    )
    .await?;
    assert_eq!(status, reqwest::StatusCode::CONFLICT);
    let Message::ErrorResponse(error) = decode(&conflict_bytes)?.message else {
        return Err("wrong conflict response".into());
    };
    assert_eq!(error.code, ErrorCode::Conflict);

    let concurrent = CreatePostV2Request {
        actor_id: actor,
        client_post_id: ClientPostId(Uuid::new_v4()),
        coordinate,
        kind: PostKind::General,
        body: format!("parallel {}", Uuid::new_v4()),
    };
    let mut tasks = JoinSet::new();
    for _ in 0..20 {
        let url = url.clone();
        let request = concurrent.clone();
        tasks.spawn(async move {
            let db = Storage::connect(&url).await?;
            db.create_post_v2(&request).await
        });
    }
    let mut parallel_response: Option<CreatePostResponse> = None;
    while let Some(result) = tasks.join_next().await {
        let response = result??;
        if let Some(expected) = &parallel_response {
            assert_eq!(&response, expected);
        } else {
            parallel_response = Some(response);
        }
    }
    let parallel_response = parallel_response.ok_or("no parallel results")?;
    assert_eq!(parallel_response.cell_revision, before_revision + 2);
    let after = Storage::connect(&url)
        .await?
        .query_cells(&[CellQuery {
            cell_id: cell,
            known_revision: 0,
        }])
        .await?;
    let CellResult::Snapshot {
        revision: after_revision,
        posts: after_posts,
        ..
    } = &after.cells[0]
    else {
        return Err("missing final snapshot".into());
    };
    assert_eq!(*after_revision, before_revision + 2);
    assert_eq!(after_posts.len(), before_count + 2);
    assert_eq!(
        after_posts
            .iter()
            .filter(|post| post.id == parallel_response.post_id)
            .count(),
        1
    );

    let started = Instant::now();
    let mut store = MemoryStore::default();
    let first_actor = load_or_create_actor(&mut store)?;
    let actor_first_us = started.elapsed().as_micros();
    let started = Instant::now();
    assert_eq!(load_or_create_actor(&mut store)?, first_actor);
    let actor_load_us = started.elapsed().as_micros();
    let started = Instant::now();
    let _ = degrees_to_coordinate(37.5665, 126.978)?;
    let e7_us = started.elapsed().as_micros();
    let now = now_ms()?;
    let fix = LocationFix {
        coordinate,
        horizontal_accuracy_m: 15.0,
        captured_at_ms: now,
    };
    let transport = HttpTransport::new(&format!("http://{address}/"))?;
    let mut runtime = ClientRuntime::new(
        &mut store,
        FakeLocation(LocationStatus::Ready(fix)),
        transport,
    )?;
    let started = Instant::now();
    let runtime_post = runtime
        .post(&format!("runtime {}", Uuid::new_v4()), now)
        .await?;
    let runtime_us = started.elapsed().as_micros();
    runtime.camera_idle(coordinate, 5_000, 0);
    assert!(runtime.refresh_visible_cells(400).await?);
    assert!(
        runtime
            .core()
            .cache()
            .get(&cell)
            .ok_or("runtime cache miss")?
            .posts
            .iter()
            .any(|post| post.id == runtime_post.post_id)
    );
    println!(
        "V02 E2E PASS before_revision={} after_revision={} before_count={} after_count={} parallel_replays=20 first_request_bytes={} first_response_bytes={} replay_response_bytes={} actor_first_us={} actor_load_us={} e7_us={} encode_10k_us={} decode_10k_us={} first_http_us={} replay_http_us={} runtime_post_us={}",
        before_revision,
        after_revision,
        before_count,
        after_posts.len(),
        first_frame_len(&first_frame)?,
        first_bytes.len(),
        replay_bytes.len(),
        actor_first_us,
        actor_load_us,
        e7_us,
        encode_10k_us,
        decode_10k_us,
        first_us,
        replay_us,
        runtime_us
    );
    server.abort();
    Ok(())
}

fn first_frame_len(frame: &Frame) -> Result<usize, mappa_protocol::ProtocolError> {
    Ok(encode(frame)?.len())
}
