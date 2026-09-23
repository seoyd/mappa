use mappa_client_core::{ClientCore, ClientError};
use mappa_client_transport::{Endpoint, Transport, TransportError};
use mappa_device::{
    ActorStore, DeviceError, LocationProvider, LocationStatus, load_or_create_actor,
    posting_coordinate,
};
use mappa_domain::{ActorId, ClientPostId, Coordinate, PostKind, validate_body};
use mappa_protocol::{
    CreatePostResponse, CreatePostV2Request, Frame, Message, ProtocolError, decode, encode,
};
use mappa_spatial::{SpatialError, coordinate_to_cell};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeState {
    Ready,
    Locating,
    Posting,
    Posted,
    Offline,
    Error,
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error(transparent)]
    Device(#[from] DeviceError),
    #[error(transparent)]
    Domain(#[from] mappa_domain::DomainError),
    #[error(transparent)]
    Spatial(#[from] SpatialError),
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error(transparent)]
    Client(#[from] ClientError),
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error("conflicting replay")]
    Conflict,
    #[error("post already in flight")]
    InFlight,
    #[error("unexpected server response")]
    UnexpectedResponse,
    #[error("device clock unavailable")]
    Clock,
}

struct PendingPost {
    client_post_id: ClientPostId,
    coordinate: Coordinate,
    body: String,
    response: Option<CreatePostResponse>,
}

pub struct ClientRuntime<L, T> {
    actor_id: ActorId,
    location: L,
    transport: T,
    core: ClientCore,
    state: RuntimeState,
    location_status: LocationStatus,
    next_request_id: u32,
    pending: Option<PendingPost>,
}

impl<L: LocationProvider, T: Transport> ClientRuntime<L, T> {
    pub fn new(
        store: &mut impl ActorStore,
        location: L,
        transport: T,
    ) -> Result<Self, RuntimeError> {
        let actor_id = load_or_create_actor(store)?;
        Ok(Self {
            actor_id,
            location,
            transport,
            core: ClientCore::default(),
            state: RuntimeState::Ready,
            location_status: LocationStatus::Unknown,
            next_request_id: 1,
            pending: None,
        })
    }
    pub fn actor_id(&self) -> ActorId {
        self.actor_id
    }
    pub fn state(&self) -> RuntimeState {
        self.state
    }
    pub fn acknowledge_post(&mut self) {
        if self.state == RuntimeState::Posted {
            self.pending = None;
            self.state = RuntimeState::Ready;
        }
    }
    pub fn replace_transport(&mut self, transport: T) {
        self.transport = transport;
        self.core = ClientCore::default();
        if self.state != RuntimeState::Posting {
            self.state = RuntimeState::Ready;
        }
    }
    pub fn location_status(&self) -> LocationStatus {
        self.location_status
    }
    pub fn core(&self) -> &ClientCore {
        &self.core
    }
    fn request_id(&mut self) -> u32 {
        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.wrapping_add(1);
        id
    }
    pub async fn refresh_location(&mut self) -> LocationStatus {
        self.state = RuntimeState::Locating;
        self.location_status = self.location.request_location().await;
        self.state = RuntimeState::Ready;
        self.location_status
    }
    pub async fn post(
        &mut self,
        body: &str,
        now_ms: i64,
    ) -> Result<CreatePostResponse, RuntimeError> {
        self.post_with_clock(body, || Ok(now_ms)).await
    }
    pub async fn post_with_clock(
        &mut self,
        body: &str,
        clock: impl FnOnce() -> Result<i64, RuntimeError>,
    ) -> Result<CreatePostResponse, RuntimeError> {
        if self.state == RuntimeState::Posting {
            return Err(RuntimeError::InFlight);
        }
        validate_body(body)?;
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.body != body)
        {
            self.pending = None;
        }
        if let Some(response) = self
            .pending
            .as_ref()
            .and_then(|pending| pending.response.clone())
        {
            return Ok(response);
        }
        if self.pending.is_none() {
            self.refresh_location().await;
            let coordinate = posting_coordinate(self.location_status, clock()?)?;
            coordinate_to_cell(coordinate)?;
            self.pending = Some(PendingPost {
                client_post_id: ClientPostId(Uuid::new_v4()),
                coordinate,
                body: body.to_owned(),
                response: None,
            });
        }
        let pending = self
            .pending
            .as_ref()
            .ok_or(RuntimeError::UnexpectedResponse)?;
        let expected_cell = coordinate_to_cell(pending.coordinate)?;
        let client_post_id = pending.client_post_id;
        let coordinate = pending.coordinate;
        let body = pending.body.clone();
        let frame = Frame {
            request_id: self.request_id(),
            message: Message::CreatePostV2Request(CreatePostV2Request {
                actor_id: self.actor_id,
                client_post_id,
                coordinate,
                kind: PostKind::General,
                body,
            }),
        };
        let bytes = encode(&frame)?;
        self.state = RuntimeState::Posting;
        let response_bytes = match self.transport.send(Endpoint::Create, bytes).await {
            Ok(bytes) => bytes,
            Err(TransportError::Status(409)) => {
                self.state = RuntimeState::Error;
                return Err(RuntimeError::Conflict);
            }
            Err(error) => {
                self.state = RuntimeState::Offline;
                return Err(RuntimeError::Transport(error));
            }
        };
        let response = match decode(&response_bytes) {
            Ok(response) => response,
            Err(error) => {
                self.state = RuntimeState::Error;
                return Err(RuntimeError::Protocol(error));
            }
        };
        let Message::CreatePostV2Response(created) = response.message else {
            self.state = RuntimeState::Error;
            return Err(RuntimeError::UnexpectedResponse);
        };
        if response.request_id != frame.request_id || created.cell_id != expected_cell {
            self.state = RuntimeState::Error;
            return Err(RuntimeError::UnexpectedResponse);
        }
        self.core.invalidate_cell(expected_cell);
        if let Some(pending) = &mut self.pending {
            pending.response = Some(created.clone());
        }
        self.state = RuntimeState::Posted;
        Ok(created)
    }
    pub fn camera_moving(&mut self) {
        self.core.camera_moving();
    }
    pub fn camera_idle(&mut self, center: Coordinate, viewport_width_meters: u32, now_ms: u64) {
        self.core.camera_idle(center, viewport_width_meters, now_ms);
    }
    pub async fn refresh_visible_cells(&mut self, now_ms: u64) -> Result<bool, RuntimeError> {
        let request_id = self.request_id();
        let Some(request) = self.core.tick(now_ms, request_id)? else {
            return Ok(false);
        };
        let Message::QueryCellsRequest(query) = decode(&request)?.message else {
            return Err(RuntimeError::UnexpectedResponse);
        };
        let response = match self.transport.send(Endpoint::Query, request).await {
            Ok(response) => response,
            Err(error) => {
                let cells: Vec<_> = query.cells.iter().map(|cell| cell.cell_id).collect();
                self.core.cancel_in_flight(&cells);
                return Err(RuntimeError::Transport(error));
            }
        };
        let cells: Vec<_> = query.cells.iter().map(|cell| cell.cell_id).collect();
        let frame = match decode(&response) {
            Ok(frame) => frame,
            Err(error) => {
                self.core.cancel_in_flight(&cells);
                return Err(RuntimeError::Protocol(error));
            }
        };
        if frame.request_id != request_id {
            self.core.cancel_in_flight(&cells);
            return Err(RuntimeError::UnexpectedResponse);
        }
        if let Err(error) = self.core.apply_query_response(&response, now_ms) {
            self.core.cancel_in_flight(&cells);
            return Err(RuntimeError::Client(error));
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mappa_device::LocationFix;
    use mappa_domain::{PostId, Timestamp};
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicI64, Ordering},
    };
    #[derive(Default)]
    struct Store(Option<Vec<u8>>);
    impl ActorStore for Store {
        fn load(&mut self) -> Result<Option<Vec<u8>>, DeviceError> {
            Ok(self.0.clone())
        }
        fn save(&mut self, bytes: &[u8; 16]) -> Result<(), DeviceError> {
            self.0 = Some(bytes.to_vec());
            Ok(())
        }
    }
    struct Location(LocationStatus);
    impl LocationProvider for Location {
        async fn request_location(&mut self) -> LocationStatus {
            self.0
        }
    }
    struct LocationAfterRequest {
        fix: LocationFix,
        clock: Arc<AtomicI64>,
    }
    impl LocationProvider for LocationAfterRequest {
        async fn request_location(&mut self) -> LocationStatus {
            self.clock
                .store(self.fix.captured_at_ms + 1, Ordering::SeqCst);
            LocationStatus::Ready(self.fix)
        }
    }
    #[derive(Clone)]
    struct FakeTransport {
        sent: Arc<Mutex<Vec<Frame>>>,
        fail: bool,
    }
    impl Transport for FakeTransport {
        async fn send(
            &self,
            endpoint: Endpoint,
            bytes: Vec<u8>,
        ) -> Result<Vec<u8>, TransportError> {
            assert_eq!(endpoint, Endpoint::Create);
            let frame = decode(&bytes).expect("runtime must encode a frame");
            let Message::CreatePostV2Request(ref request) = frame.message else {
                panic!("expected v2 create");
            };
            let cell = coordinate_to_cell(request.coordinate).expect("valid cell");
            self.sent.lock().expect("test mutex").push(frame.clone());
            if self.fail {
                return Err(TransportError::Status(500));
            }
            encode(&Frame {
                request_id: frame.request_id,
                message: Message::CreatePostV2Response(CreatePostResponse {
                    post_id: PostId(Uuid::from_u128(1)),
                    cell_id: cell,
                    cell_revision: 1,
                    created_at: Timestamp(1000),
                }),
            })
            .map_err(|_| TransportError::TooLarge)
        }
    }
    #[tokio::test]
    async fn runtime_posts_once_and_reuses_logical_post() {
        let fix = LocationFix {
            coordinate: Coordinate::new(0, 0).unwrap(),
            horizontal_accuracy_m: 20.0,
            captured_at_ms: 1000,
        };
        let sent = Arc::new(Mutex::new(Vec::new()));
        let mut store = Store::default();
        let mut runtime = ClientRuntime::new(
            &mut store,
            Location(LocationStatus::Ready(fix)),
            FakeTransport {
                sent: sent.clone(),
                fail: false,
            },
        )
        .unwrap();
        let actor = runtime.actor_id();
        let first = runtime.post("hello", 1001).await.unwrap();
        let second = runtime.post("hello", 1002).await.unwrap();
        assert_eq!(first, second);
        assert_eq!(runtime.state(), RuntimeState::Posted);
        assert_eq!(sent.lock().unwrap().len(), 1);
        runtime.acknowledge_post();
        runtime.post("hello", 1003).await.unwrap();
        let frames = sent.lock().unwrap();
        assert_eq!(frames.len(), 2);
        let (Message::CreatePostV2Request(a), Message::CreatePostV2Request(b)) =
            (&frames[0].message, &frames[1].message)
        else {
            panic!("expected two v2 requests");
        };
        assert_ne!(a.client_post_id, b.client_post_id);
        drop(frames);
        let restarted = ClientRuntime::new(
            &mut store,
            Location(LocationStatus::Ready(fix)),
            FakeTransport { sent, fail: false },
        )
        .unwrap();
        assert_eq!(restarted.actor_id(), actor);
    }
    #[tokio::test]
    async fn invalid_locations_never_send() {
        let fix = LocationFix {
            coordinate: Coordinate::new(0, 0).unwrap(),
            horizontal_accuracy_m: 20.0,
            captured_at_ms: 1000,
        };
        for status in [
            LocationStatus::Denied,
            LocationStatus::Ready(fix),
            LocationStatus::Ready(LocationFix {
                horizontal_accuracy_m: 101.0,
                ..fix
            }),
        ] {
            let sent = Arc::new(Mutex::new(Vec::new()));
            let mut runtime = ClientRuntime::new(
                &mut Store::default(),
                Location(status),
                FakeTransport {
                    sent: sent.clone(),
                    fail: false,
                },
            )
            .unwrap();
            assert!(runtime.post("hello", 32_000).await.is_err());
            assert!(sent.lock().unwrap().is_empty());
        }
    }
    #[tokio::test]
    async fn validates_one_shot_fix_after_location_returns() {
        let clock = Arc::new(AtomicI64::new(1_000));
        let sent = Arc::new(Mutex::new(Vec::new()));
        let mut runtime = ClientRuntime::new(
            &mut Store::default(),
            LocationAfterRequest {
                fix: LocationFix {
                    coordinate: Coordinate::new(0, 0).unwrap(),
                    horizontal_accuracy_m: 20.0,
                    captured_at_ms: 2_000,
                },
                clock: clock.clone(),
            },
            FakeTransport {
                sent: sent.clone(),
                fail: false,
            },
        )
        .unwrap();
        runtime
            .post_with_clock("fresh fix", || Ok(clock.load(Ordering::SeqCst)))
            .await
            .unwrap();
        assert_eq!(sent.lock().unwrap().len(), 1);
    }
    #[tokio::test]
    async fn url_replacement_preserves_pending_retry_identity() {
        let fix = LocationFix {
            coordinate: Coordinate::new(0, 0).unwrap(),
            horizontal_accuracy_m: 20.0,
            captured_at_ms: 1000,
        };
        let sent = Arc::new(Mutex::new(Vec::new()));
        let mut runtime = ClientRuntime::new(
            &mut Store::default(),
            Location(LocationStatus::Ready(fix)),
            FakeTransport {
                sent: sent.clone(),
                fail: true,
            },
        )
        .unwrap();
        let actor = runtime.actor_id();
        assert!(matches!(
            runtime.post("retry me", 1001).await,
            Err(RuntimeError::Transport(TransportError::Status(500)))
        ));
        assert_eq!(runtime.state(), RuntimeState::Offline);
        runtime.replace_transport(FakeTransport {
            sent: sent.clone(),
            fail: false,
        });
        runtime.post("retry me", 1002).await.unwrap();
        let frames = sent.lock().unwrap();
        let (Message::CreatePostV2Request(first), Message::CreatePostV2Request(second)) =
            (&frames[0].message, &frames[1].message)
        else {
            panic!("expected v2 frames")
        };
        assert_eq!(first.client_post_id, second.client_post_id);
        assert_eq!(first.actor_id, actor);
        assert_eq!(second.actor_id, actor);
    }
}
