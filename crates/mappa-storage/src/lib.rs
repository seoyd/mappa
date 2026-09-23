use mappa_domain::{ActorId, CellId, Coordinate, Post, PostId, PostKind, Timestamp, validate_body};
use mappa_protocol::{
    CellQuery, CellResult, CreatePostRequest, CreatePostResponse, CreatePostV2Request,
    QueryCellsResponse,
};
use mappa_spatial::coordinate_to_cell;
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use tokio::sync::Mutex;
use tokio_postgres::{Client, NoTls};
use uuid::Uuid;

#[derive(Clone)]
pub struct Storage {
    client: Arc<Mutex<Client>>,
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] tokio_postgres::Error),
    #[error("invalid input or stored data")]
    Invalid,
    #[error("system clock error")]
    Clock,
    #[error("client post id conflicts with a different post")]
    Conflict,
}

fn now_ms() -> Result<i64, StorageError> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StorageError::Clock)?
        .as_millis();
    i64::try_from(millis).map_err(|_| StorageError::Clock)
}

fn db_cell(id: CellId) -> Result<i64, StorageError> {
    i64::try_from(id.0).map_err(|_| StorageError::Invalid)
}
impl Storage {
    pub async fn connect(url: &str) -> Result<Self, StorageError> {
        let (client, connection) = tokio_postgres::connect(url, NoTls).await?;
        tokio::spawn(async move {
            if let Err(error) = connection.await {
                tracing::error!(%error, "database connection closed");
            }
        });
        Ok(Self {
            client: Arc::new(Mutex::new(client)),
        })
    }

    pub async fn migrate(&self) -> Result<(), StorageError> {
        let mut client = self.client.lock().await;
        let tx = client.transaction().await?;
        tx.batch_execute(include_str!("../../../migrations/0001_initial.sql"))
            .await?;
        tx.batch_execute(include_str!("../../../migrations/0002_client_post_id.sql"))
            .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn create_post_v2(
        &self,
        request: &CreatePostV2Request,
    ) -> Result<CreatePostResponse, StorageError> {
        validate_body(&request.body).map_err(|_| StorageError::Invalid)?;
        let cell_id = coordinate_to_cell(request.coordinate).map_err(|_| StorageError::Invalid)?;
        let cell = db_cell(cell_id)?;
        let id = Uuid::new_v4();
        let created_at = now_ms()?;
        let mut client = self.client.lock().await;
        let tx = client.transaction().await?;
        let inserted = tx.query_opt(
            "INSERT INTO posts (id, actor_id, client_post_id, cell_id, lat_e7, lon_e7, geom, kind, body, created_at) VALUES ($1,$2,$3,$4,$5,$6,ST_SetSRID(ST_MakePoint($6::integer::double precision / 10000000.0, $5::integer::double precision / 10000000.0),4326)::geography,$7,$8,$9) ON CONFLICT (actor_id, client_post_id) WHERE client_post_id IS NOT NULL DO NOTHING RETURNING id",
            &[&id, &request.actor_id.0, &request.client_post_id.0, &cell, &request.coordinate.lat_e7, &request.coordinate.lon_e7, &(request.kind as i16), &request.body, &created_at]
        ).await?;
        if inserted.is_some() {
            let row = tx.query_one(
                "INSERT INTO cell_state (cell_id, revision, updated_at) VALUES ($1,1,$2) ON CONFLICT (cell_id) DO UPDATE SET revision = cell_state.revision + 1, updated_at = EXCLUDED.updated_at RETURNING revision",
                &[&cell, &created_at]
            ).await?;
            let revision: i64 = row.get(0);
            tx.execute(
                "UPDATE posts SET create_revision=$2 WHERE id=$1",
                &[&id, &revision],
            )
            .await?;
            tx.commit().await?;
            return Ok(CreatePostResponse {
                post_id: PostId(id),
                cell_id,
                cell_revision: u64::try_from(revision).map_err(|_| StorageError::Invalid)?,
                created_at: Timestamp(created_at),
            });
        }
        let row = tx.query_one("SELECT id, cell_id, lat_e7, lon_e7, kind, body, created_at, create_revision FROM posts WHERE actor_id=$1 AND client_post_id=$2", &[&request.actor_id.0, &request.client_post_id.0]).await?;
        let stored_cell: i64 = row.get(1);
        let stored_lat: i32 = row.get(2);
        let stored_lon: i32 = row.get(3);
        let stored_kind: i16 = row.get(4);
        let stored_body: String = row.get(5);
        if stored_cell != cell
            || stored_lat != request.coordinate.lat_e7
            || stored_lon != request.coordinate.lon_e7
            || stored_kind != request.kind as i16
            || stored_body != request.body
        {
            return Err(StorageError::Conflict);
        }
        let revision: i64 = row.get::<_, Option<i64>>(7).ok_or(StorageError::Invalid)?;
        let response = CreatePostResponse {
            post_id: PostId(row.get(0)),
            cell_id,
            cell_revision: u64::try_from(revision).map_err(|_| StorageError::Invalid)?,
            created_at: Timestamp(row.get(6)),
        };
        tx.commit().await?;
        Ok(response)
    }

    pub async fn create_post(
        &self,
        request: &CreatePostRequest,
    ) -> Result<CreatePostResponse, StorageError> {
        validate_body(&request.body).map_err(|_| StorageError::Invalid)?;
        let cell_id = coordinate_to_cell(request.coordinate).map_err(|_| StorageError::Invalid)?;
        let cell = db_cell(cell_id)?;
        let id = Uuid::new_v4();
        let created_at = now_ms()?;
        let mut client = self.client.lock().await;
        let tx = client.transaction().await?;
        tx.execute(
            "INSERT INTO posts (id, actor_id, cell_id, lat_e7, lon_e7, geom, kind, body, created_at) VALUES ($1,$2,$3,$4,$5,ST_SetSRID(ST_MakePoint($5::integer::double precision / 10000000.0, $4::integer::double precision / 10000000.0),4326)::geography,$6,$7,$8)",
            &[&id, &request.actor_id.0, &cell, &request.coordinate.lat_e7, &request.coordinate.lon_e7, &(request.kind as i16), &request.body, &created_at]
        ).await?;
        let row = tx.query_one(
            "INSERT INTO cell_state (cell_id, revision, updated_at) VALUES ($1,1,$2) ON CONFLICT (cell_id) DO UPDATE SET revision = cell_state.revision + 1, updated_at = EXCLUDED.updated_at RETURNING revision",
            &[&cell, &created_at]
        ).await?;
        let revision: i64 = row.get(0);
        tx.commit().await?;
        Ok(CreatePostResponse {
            post_id: PostId(id),
            cell_id,
            cell_revision: u64::try_from(revision).map_err(|_| StorageError::Invalid)?,
            created_at: Timestamp(created_at),
        })
    }

    pub async fn query_cells(
        &self,
        cells: &[CellQuery],
    ) -> Result<QueryCellsResponse, StorageError> {
        if cells.is_empty() || cells.len() > mappa_protocol::MAX_QUERY_CELLS {
            return Err(StorageError::Invalid);
        }
        let mut client = self.client.lock().await;
        let tx = client
            .build_transaction()
            .isolation_level(tokio_postgres::IsolationLevel::RepeatableRead)
            .read_only(true)
            .start()
            .await?;
        let mut result = Vec::with_capacity(cells.len());
        for query in cells {
            let cell = db_cell(query.cell_id)?;
            let revision_row = tx
                .query_opt("SELECT revision FROM cell_state WHERE cell_id=$1", &[&cell])
                .await?;
            let revision = revision_row.map(|row| row.get::<_, i64>(0)).unwrap_or(0);
            let revision = u64::try_from(revision).map_err(|_| StorageError::Invalid)?;
            if revision != 0 && revision == query.known_revision {
                result.push(CellResult::Unchanged {
                    cell_id: query.cell_id,
                    revision,
                });
                continue;
            }
            let rows = tx.query("SELECT id, actor_id, lat_e7, lon_e7, kind, body, created_at, expires_at FROM posts WHERE cell_id=$1 AND (expires_at IS NULL OR expires_at > $2) ORDER BY created_at, id", &[&cell, &now_ms()?]).await?;
            let mut posts = Vec::with_capacity(rows.len());
            for row in rows {
                let lat: i32 = row.get(2);
                let lon: i32 = row.get(3);
                let kind: i16 = row.get(4);
                posts.push(Post {
                    id: PostId(row.get(0)),
                    actor_id: ActorId(row.get(1)),
                    coordinate: Coordinate::new(lat, lon).map_err(|_| StorageError::Invalid)?,
                    cell_id: query.cell_id,
                    kind: PostKind::try_from(
                        u8::try_from(kind).map_err(|_| StorageError::Invalid)?,
                    )
                    .map_err(|_| StorageError::Invalid)?,
                    body: row.get(5),
                    created_at: Timestamp(row.get(6)),
                    expires_at: row.get::<_, Option<i64>>(7).map(Timestamp),
                });
            }
            result.push(CellResult::Snapshot {
                cell_id: query.cell_id,
                revision,
                posts,
            });
        }
        tx.commit().await?;
        Ok(QueryCellsResponse { cells: result })
    }
}
