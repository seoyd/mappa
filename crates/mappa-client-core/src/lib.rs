use mappa_domain::{ActorId, CellId, Coordinate, Post, PostKind};
use mappa_protocol::{
    CellQuery, CellResult, CreatePostRequest, Frame, Message, ProtocolError, QueryCellsRequest,
    QueryCellsResponse, decode, encode,
};
use mappa_spatial::{SpatialError, coordinate_to_cell, neighbors_3x3};
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use uuid::Uuid;

pub const LOCAL_DETAIL_MAX_VIEWPORT_METERS: u32 = 6000;
pub const CAMERA_IDLE_DEBOUNCE_MS: u64 = 400;
pub const CACHE_STALE_AFTER_MS: u64 = 60_000;

#[derive(Clone, Debug)]
pub struct CellCacheEntry {
    pub cell_id: CellId,
    pub revision: u64,
    pub posts: Vec<Post>,
    pub last_updated_ms: u64,
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error(transparent)]
    Spatial(#[from] SpatialError),
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error("unexpected response")]
    UnexpectedResponse,
}

#[derive(Default)]
pub struct ClientCore {
    cache: HashMap<CellId, CellCacheEntry>,
    in_flight: HashSet<CellId>,
    pending_idle: Option<(Coordinate, u32, u64)>,
}

impl ClientCore {
    pub fn cache(&self) -> &HashMap<CellId, CellCacheEntry> {
        &self.cache
    }
    pub fn camera_moving(&mut self) {
        self.pending_idle = None;
    }
    pub fn camera_idle(&mut self, center: Coordinate, viewport_width_meters: u32, now_ms: u64) {
        self.pending_idle = Some((center, viewport_width_meters, now_ms));
    }
    pub fn tick(&mut self, now_ms: u64, request_id: u32) -> Result<Option<Vec<u8>>, ClientError> {
        let Some((center, width, idle_at)) = self.pending_idle else {
            return Ok(None);
        };
        if now_ms.saturating_sub(idle_at) < CAMERA_IDLE_DEBOUNCE_MS {
            return Ok(None);
        }
        self.pending_idle = None;
        if width > LOCAL_DETAIL_MAX_VIEWPORT_METERS {
            return Ok(None);
        }
        let cells = neighbors_3x3(coordinate_to_cell(center)?)?;
        let missing: Vec<CellQuery> = cells
            .into_iter()
            .filter_map(|cell_id| {
                if self.in_flight.contains(&cell_id) {
                    return None;
                }
                match self.cache.get(&cell_id) {
                    Some(entry)
                        if now_ms.saturating_sub(entry.last_updated_ms) < CACHE_STALE_AFTER_MS =>
                    {
                        None
                    }
                    Some(entry) => Some(CellQuery {
                        cell_id,
                        known_revision: entry.revision,
                    }),
                    None => Some(CellQuery {
                        cell_id,
                        known_revision: 0,
                    }),
                }
            })
            .collect();
        if missing.is_empty() {
            return Ok(None);
        }
        let bytes = encode(&Frame {
            request_id,
            message: Message::QueryCellsRequest(QueryCellsRequest {
                cells: missing.clone(),
            }),
        })?;
        self.in_flight
            .extend(missing.into_iter().map(|cell| cell.cell_id));
        Ok(Some(bytes))
    }
    pub fn apply_query_response(&mut self, bytes: &[u8], now_ms: u64) -> Result<(), ClientError> {
        let frame = decode(bytes)?;
        let Message::QueryCellsResponse(QueryCellsResponse { cells }) = frame.message else {
            return Err(ClientError::UnexpectedResponse);
        };
        for cell in cells {
            match cell {
                CellResult::Unchanged { cell_id, revision } => {
                    let entry = self
                        .cache
                        .get_mut(&cell_id)
                        .ok_or(ClientError::UnexpectedResponse)?;
                    if entry.revision != revision {
                        return Err(ClientError::UnexpectedResponse);
                    }
                    entry.last_updated_ms = now_ms;
                    self.in_flight.remove(&cell_id);
                }
                CellResult::Snapshot {
                    cell_id,
                    revision,
                    posts,
                } => {
                    self.cache.insert(
                        cell_id,
                        CellCacheEntry {
                            cell_id,
                            revision,
                            posts,
                            last_updated_ms: now_ms,
                        },
                    );
                    self.in_flight.remove(&cell_id);
                }
            }
        }
        Ok(())
    }
    pub fn cancel_in_flight(&mut self, cells: &[CellId]) {
        for cell in cells {
            self.in_flight.remove(cell);
        }
    }
    pub fn create_request(
        actor_id: ActorId,
        coordinate: Coordinate,
        body: String,
        request_id: u32,
    ) -> Result<Vec<u8>, ClientError> {
        Ok(encode(&Frame {
            request_id,
            message: Message::CreatePostRequest(CreatePostRequest {
                actor_id,
                coordinate,
                kind: PostKind::General,
                body,
            }),
        })?)
    }
    pub fn new_actor_id() -> ActorId {
        ActorId(Uuid::new_v4())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn center() -> Coordinate {
        Coordinate::new(375_665_000, 1_269_780_000).unwrap()
    }
    #[test]
    fn no_pan_and_cache() {
        let mut core = ClientCore::default();
        for _ in 0..50 {
            core.camera_moving();
            assert!(core.tick(1000, 1).unwrap().is_none());
        }
        core.camera_idle(center(), 50_000, 1000);
        assert!(core.tick(1400, 1).unwrap().is_none());
        core.camera_idle(center(), 5_000, 1500);
        assert!(core.tick(1800, 1).unwrap().is_none());
        let request = core.tick(1900, 1).unwrap().unwrap();
        let Message::QueryCellsRequest(q) = decode(&request).unwrap().message else {
            panic!()
        };
        assert_eq!(q.cells.len(), 9);
        core.camera_idle(center(), 5_000, 2000);
        assert!(core.tick(2400, 2).unwrap().is_none()); // all in flight
        let reply = encode(&Frame {
            request_id: 1,
            message: Message::QueryCellsResponse(QueryCellsResponse {
                cells: q
                    .cells
                    .iter()
                    .map(|c| CellResult::Snapshot {
                        cell_id: c.cell_id,
                        revision: 0,
                        posts: vec![],
                    })
                    .collect(),
            }),
        })
        .unwrap();
        core.apply_query_response(&reply, 2400).unwrap();
        core.camera_idle(center(), 5_000, 2500);
        assert!(core.tick(2900, 3).unwrap().is_none()); // cache hit
        let (_, x, _) = mappa_spatial::cell_to_xy(coordinate_to_cell(center()).unwrap()).unwrap();
        let next_lon_e7 = (((f64::from(x + 1) + 0.5)
            / f64::from(1_u32 << mappa_spatial::CONTENT_CELL_ZOOM)
            * 360.0
            - 180.0)
            * 10_000_000.0)
            .round() as i32;
        let next = Coordinate::new(center().lat_e7, next_lon_e7).unwrap();
        core.camera_idle(next, 5_000, 3000);
        let shifted = core.tick(3400, 4).unwrap().unwrap();
        let Message::QueryCellsRequest(query) = decode(&shifted).unwrap().message else {
            panic!()
        };
        assert_eq!(query.cells.len(), 3); // only the newly entered column
    }
}
