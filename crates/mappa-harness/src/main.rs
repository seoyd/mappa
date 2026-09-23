use mappa_client_core::ClientCore;
use mappa_domain::{ActorId, CellId, Coordinate, Post, PostId, PostKind, Timestamp};
use mappa_protocol::{
    CONTENT_TYPE, CellQuery, CellResult, CreatePostRequest, CreatePostResponse, Frame, Message,
    QueryCellsRequest, QueryCellsResponse, decode, encode,
};
use mappa_spatial::{coordinate_to_cell, neighbors_3x3};
use mappa_storage::Storage;
use std::{env, error::Error, time::Instant};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let database_url = env::var("DATABASE_URL")?;
    let storage = Storage::connect(&database_url).await?;
    storage.migrate().await?;
    let center = Coordinate::new(375_665_000, 1_269_780_000)?;
    let cell = coordinate_to_cell(center)?;
    let baseline = storage
        .query_cells(&[CellQuery {
            cell_id: cell,
            known_revision: 0,
        }])
        .await?;
    let CellResult::Snapshot {
        revision: before_revision,
        posts: before_posts,
        ..
    } = &baseline.cells[0]
    else {
        return Err("expected baseline snapshot".into());
    };
    let before_revision = *before_revision;
    let before_count = before_posts.len();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let app = mappa_server::router(storage.clone());
    let server = tokio::spawn(async move { axum::serve(listener, app).await });
    let http = reqwest::Client::new();
    let actor = ClientCore::new_actor_id();
    let body = format!("Mappa harness {}", uuid::Uuid::new_v4());
    let create_bytes = ClientCore::create_request(actor, center, body.clone(), 1)?;
    let created_raw = http
        .post(format!("http://{address}/v1/posts"))
        .header("content-type", CONTENT_TYPE)
        .body(create_bytes.clone())
        .send()
        .await?;
    assert_eq!(created_raw.status(), reqwest::StatusCode::CREATED);
    let created_bytes = created_raw.bytes().await?;
    let Message::CreatePostResponse(created) = decode(&created_bytes)?.message else {
        return Err("wrong create response".into());
    };
    assert_eq!(created.cell_id, cell);
    assert_eq!(created.cell_revision, before_revision + 1);
    let malformed = http
        .post(format!("http://{address}/v1/posts"))
        .header("content-type", CONTENT_TYPE)
        .body(vec![0_u8; 16])
        .send()
        .await?;
    assert_eq!(malformed.status(), reqwest::StatusCode::BAD_REQUEST);
    assert!(matches!(
        decode(&malformed.bytes().await?)?.message,
        Message::ErrorResponse(_)
    ));
    let oversized = http
        .post(format!("http://{address}/v1/posts"))
        .header("content-type", CONTENT_TYPE)
        .body(vec![0_u8; 65_537])
        .send()
        .await?;
    assert_eq!(oversized.status(), reqwest::StatusCode::PAYLOAD_TOO_LARGE);
    assert!(matches!(
        decode(&oversized.bytes().await?)?.message,
        Message::ErrorResponse(_)
    ));
    let mut client_b = ClientCore::default();
    client_b.camera_idle(center, 5_000, 0);
    let batch = client_b.tick(400, 2)?.ok_or("missing batch request")?;
    let Message::QueryCellsRequest(batch_query) = decode(&batch)?.message else {
        return Err("wrong client query".into());
    };
    assert_eq!(batch_query.cells.len(), 9);
    let queried = http
        .post(format!("http://{address}/v1/cells/query"))
        .header("content-type", CONTENT_TYPE)
        .body(batch)
        .send()
        .await?;
    assert_eq!(queried.status(), reqwest::StatusCode::OK);
    let query_bytes = queried.bytes().await?;
    client_b.apply_query_response(&query_bytes, 400)?;
    let cached = client_b.cache().get(&cell).ok_or("missing cached cell")?;
    assert_eq!(cached.revision, before_revision + 1);
    assert_eq!(cached.posts.len(), before_count + 1);
    assert!(
        cached
            .posts
            .iter()
            .any(|post| post.id == created.post_id && post.body == body)
    );
    let Message::QueryCellsResponse(result) = decode(&query_bytes)?.message else {
        return Err("wrong query response".into());
    };
    let CellResult::Snapshot {
        revision, posts, ..
    } = result
        .cells
        .iter()
        .find(|result| matches!(result, CellResult::Snapshot { cell_id, .. } if *cell_id == cell))
        .ok_or("missing target cell")?
    else {
        return Err("expected snapshot".into());
    };
    assert!(
        posts
            .iter()
            .any(|post| post.id == created.post_id && post.body == body)
    );
    let same = http
        .post(format!("http://{address}/v1/cells/query"))
        .header("content-type", CONTENT_TYPE)
        .body(encode(&Frame {
            request_id: 3,
            message: Message::QueryCellsRequest(QueryCellsRequest {
                cells: vec![CellQuery {
                    cell_id: cell,
                    known_revision: *revision,
                }],
            }),
        })?)
        .send()
        .await?;
    let Message::QueryCellsResponse(unchanged) = decode(&same.bytes().await?)?.message else {
        return Err("wrong unchanged response".into());
    };
    assert!(matches!(&unchanged.cells[0], CellResult::Unchanged { .. }));
    let second = create_http(&http, address, actor, center, 4).await?;
    let third = create_http(&http, address, actor, center, 5).await?;
    let other = create_http(&http, address, actor, Coordinate::new(0, 0)?, 6).await?;
    assert_ne!(other.cell_id, cell);
    let latest = query_http(&http, address, cell, *revision, 7).await?;
    let Message::QueryCellsResponse(updated) = decode(&latest)?.message else {
        return Err("wrong updated response".into());
    };
    let CellResult::Snapshot {
        revision: final_revision,
        posts: final_posts,
        ..
    } = &updated.cells[0]
    else {
        return Err("expected updated snapshot".into());
    };
    assert_eq!(*final_revision, before_revision + 3);
    assert_eq!(final_posts.len(), before_count + 3);
    assert!(final_posts.iter().any(|post| post.id == second.post_id));
    assert!(final_posts.iter().any(|post| post.id == third.post_id));
    assert!(!final_posts.iter().any(|post| post.id == other.post_id));
    let fresh = Storage::connect(&database_url).await?;
    let persisted = fresh
        .query_cells(&[CellQuery {
            cell_id: cell,
            known_revision: 0,
        }])
        .await?;
    let CellResult::Snapshot {
        posts: persisted_posts,
        ..
    } = &persisted.cells[0]
    else {
        return Err("expected persisted snapshot".into());
    };
    assert!(
        persisted_posts
            .iter()
            .any(|post| post.id == created.post_id)
    );
    let (db, connection) = tokio_postgres::connect(&database_url, tokio_postgres::NoTls).await?;
    tokio::spawn(async move {
        let _ = connection.await;
    });
    let row = db
        .query_one(
            "SELECT ST_AsText(geom::geometry) FROM posts WHERE id=$1",
            &[&created.post_id.0],
        )
        .await?;
    let geometry: String = row.get(0);
    assert!(geometry.starts_with("POINT("));
    server.abort();
    let _ = server.await;
    let restarted_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let restarted_address = restarted_listener.local_addr()?;
    let restarted_storage = fresh.clone();
    let restarted_server = tokio::spawn(async move {
        axum::serve(restarted_listener, mappa_server::router(restarted_storage)).await
    });
    let restarted_reply = query_http(&http, restarted_address, cell, 0, 8).await?;
    let Message::QueryCellsResponse(restarted_result) = decode(&restarted_reply)?.message else {
        return Err("wrong response after restart".into());
    };
    let CellResult::Snapshot {
        posts: restarted_posts,
        ..
    } = &restarted_result.cells[0]
    else {
        return Err("expected snapshot after restart".into());
    };
    assert!(
        restarted_posts
            .iter()
            .any(|post| post.id == created.post_id)
    );
    println!(
        "E2E PASS create_request_bytes={} create_response_bytes={} query_response_bytes={} revision_before={} revision_after={} post_count_before={} post_count_after={} geometry_verified=true",
        create_bytes.len(),
        created_bytes.len(),
        query_bytes.len(),
        before_revision,
        final_revision,
        before_count,
        final_posts.len()
    );
    benchmark(&fresh, cell, center, actor).await?;
    restarted_server.abort();
    Ok(())
}

async fn create_http(
    http: &reqwest::Client,
    address: std::net::SocketAddr,
    actor: ActorId,
    coordinate: Coordinate,
    request_id: u32,
) -> Result<CreatePostResponse, Box<dyn Error>> {
    let bytes = ClientCore::create_request(
        actor,
        coordinate,
        format!("Harness {}", Uuid::new_v4()),
        request_id,
    )?;
    let response = http
        .post(format!("http://{address}/v1/posts"))
        .header("content-type", CONTENT_TYPE)
        .body(bytes)
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::CREATED);
    let Message::CreatePostResponse(created) = decode(&response.bytes().await?)?.message else {
        return Err("wrong create response".into());
    };
    Ok(created)
}

async fn query_http(
    http: &reqwest::Client,
    address: std::net::SocketAddr,
    cell_id: CellId,
    known_revision: u64,
    request_id: u32,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let bytes = encode(&Frame {
        request_id,
        message: Message::QueryCellsRequest(QueryCellsRequest {
            cells: vec![CellQuery {
                cell_id,
                known_revision,
            }],
        }),
    })?;
    let response = http
        .post(format!("http://{address}/v1/cells/query"))
        .header("content-type", CONTENT_TYPE)
        .body(bytes)
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    Ok(response.bytes().await?.to_vec())
}

async fn benchmark(
    storage: &Storage,
    cell: CellId,
    center: Coordinate,
    actor: ActorId,
) -> Result<(), Box<dyn Error>> {
    let started = Instant::now();
    for _ in 0..100_000 {
        std::hint::black_box(coordinate_to_cell(std::hint::black_box(center))?);
    }
    println!("BENCH coordinate_100k_us={}", started.elapsed().as_micros());
    let message = Frame {
        request_id: 1,
        message: Message::CreatePostRequest(CreatePostRequest {
            actor_id: actor,
            coordinate: center,
            kind: PostKind::General,
            body: "benchmark".into(),
        }),
    };
    let started = Instant::now();
    for _ in 0..10_000 {
        std::hint::black_box(encode(&message)?);
    }
    println!("BENCH encode_10k_us={}", started.elapsed().as_micros());
    let bytes = encode(&message)?;
    let started = Instant::now();
    for _ in 0..10_000 {
        std::hint::black_box(decode(&bytes)?);
    }
    println!("BENCH decode_10k_us={}", started.elapsed().as_micros());
    let mut core = ClientCore::default();
    core.camera_idle(center, 5_000, 0);
    let _ = core.tick(400, 1)?;
    let started = Instant::now();
    for _ in 0..100_000 {
        std::hint::black_box(core.cache().get(&cell));
    }
    println!(
        "BENCH cache_lookup_100k_us={}",
        started.elapsed().as_micros()
    );
    let started = Instant::now();
    for _ in 0..100 {
        std::hint::black_box(
            storage
                .query_cells(&[CellQuery {
                    cell_id: cell,
                    known_revision: 0,
                }])
                .await?,
        );
    }
    println!("BENCH db_query_100_us={}", started.elapsed().as_micros());
    for count in [100, 1000] {
        let posts: Vec<Post> = (0..count)
            .map(|_| Post {
                id: PostId(Uuid::new_v4()),
                actor_id: actor,
                coordinate: center,
                cell_id: cell,
                kind: PostKind::General,
                body: "benchmark post".into(),
                created_at: Timestamp(0),
                expires_at: None,
            })
            .collect();
        let binary = encode(&Frame {
            request_id: 1,
            message: Message::QueryCellsResponse(QueryCellsResponse {
                cells: vec![CellResult::Snapshot {
                    cell_id: cell,
                    revision: 1,
                    posts: posts.clone(),
                }],
            }),
        })?;
        let json_posts: Vec<_> = posts.iter().map(|post| serde_json::json!({"id":post.id.0.to_string(),"actor_id":post.actor_id.0.to_string(),"lat_e7":post.coordinate.lat_e7,"lon_e7":post.coordinate.lon_e7,"cell_id":post.cell_id.0,"kind":1,"body":post.body,"created_at":post.created_at.0,"expires_at":null})).collect();
        let json = serde_json::to_vec(&json_posts)?;
        println!(
            "BENCH posts={} binary_bytes={} json_baseline_bytes={}",
            count,
            binary.len(),
            json.len()
        );
    }
    let neighbors = neighbors_3x3(cell)?;
    println!("BENCH neighbor_count={}", neighbors.len());
    Ok(())
}
