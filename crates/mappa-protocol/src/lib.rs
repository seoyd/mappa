use mappa_domain::{
    ActorId, CellId, ClientPostId, Coordinate, Post, PostId, PostKind, Timestamp, validate_body,
};
use mappa_spatial::{CONTENT_CELL_ZOOM, cell_to_xy, coordinate_to_cell};
use thiserror::Error;
use uuid::Uuid;

pub const CONTENT_TYPE: &str = "application/x-mappa";
pub const MAX_QUERY_CELLS: usize = 16;
pub const MAX_PAYLOAD_BYTES: usize = 2 * 1024 * 1024;
pub const HEADER_BYTES: usize = 16;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatePostRequest {
    pub actor_id: ActorId,
    pub coordinate: Coordinate,
    pub kind: PostKind,
    pub body: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatePostV2Request {
    pub actor_id: ActorId,
    pub client_post_id: ClientPostId,
    pub coordinate: Coordinate,
    pub kind: PostKind,
    pub body: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatePostResponse {
    pub post_id: PostId,
    pub cell_id: CellId,
    pub cell_revision: u64,
    pub created_at: Timestamp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CellQuery {
    pub cell_id: CellId,
    pub known_revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryCellsRequest {
    pub cells: Vec<CellQuery>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CellResult {
    Unchanged {
        cell_id: CellId,
        revision: u64,
    },
    Snapshot {
        cell_id: CellId,
        revision: u64,
        posts: Vec<Post>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryCellsResponse {
    pub cells: Vec<CellResult>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum ErrorCode {
    InvalidRequest = 1,
    TooLarge = 2,
    Internal = 3,
    Conflict = 4,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorResponse {
    pub code: ErrorCode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Message {
    CreatePostRequest(CreatePostRequest),
    CreatePostResponse(CreatePostResponse),
    QueryCellsRequest(QueryCellsRequest),
    QueryCellsResponse(QueryCellsResponse),
    ErrorResponse(ErrorResponse),
    CreatePostV2Request(CreatePostV2Request),
    CreatePostV2Response(CreatePostResponse),
}

impl Message {
    pub fn kind(&self) -> u8 {
        match self {
            Self::CreatePostRequest(_) => 1,
            Self::CreatePostResponse(_) => 2,
            Self::QueryCellsRequest(_) => 3,
            Self::QueryCellsResponse(_) => 4,
            Self::ErrorResponse(_) => 5,
            Self::CreatePostV2Request(_) => 6,
            Self::CreatePostV2Response(_) => 7,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    pub request_id: u32,
    pub message: Message,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ProtocolError {
    #[error("truncated frame")]
    Truncated,
    #[error("invalid frame")]
    Invalid,
    #[error("unsupported protocol version")]
    UnsupportedVersion,
    #[error("frame or collection exceeds limit")]
    TooLarge,
}

fn valid_cell(cell: CellId) -> Result<(), ProtocolError> {
    let (zoom, _, _) = cell_to_xy(cell).map_err(|_| ProtocolError::Invalid)?;
    if zoom != CONTENT_CELL_ZOOM {
        return Err(ProtocolError::Invalid);
    }
    Ok(())
}

fn valid_cells(cells: &[CellQuery]) -> Result<(), ProtocolError> {
    if cells.is_empty() || cells.len() > MAX_QUERY_CELLS {
        return Err(ProtocolError::TooLarge);
    }
    let mut seen = std::collections::HashSet::new();
    for cell in cells {
        valid_cell(cell.cell_id)?;
        if !seen.insert(cell.cell_id) {
            return Err(ProtocolError::Invalid);
        }
    }
    Ok(())
}

fn push_post(out: &mut Vec<u8>, post: &Post) -> Result<(), ProtocolError> {
    valid_body(&post.body)?;
    valid_cell(post.cell_id)?;
    if coordinate_to_cell(post.coordinate).map_err(|_| ProtocolError::Invalid)? != post.cell_id {
        return Err(ProtocolError::Invalid);
    }
    out.extend_from_slice(post.id.0.as_bytes());
    out.extend_from_slice(post.actor_id.0.as_bytes());
    out.extend_from_slice(&post.coordinate.lat_e7.to_le_bytes());
    out.extend_from_slice(&post.coordinate.lon_e7.to_le_bytes());
    out.push(post.kind as u8);
    out.extend_from_slice(&(post.body.len() as u16).to_le_bytes());
    out.extend_from_slice(post.body.as_bytes());
    out.extend_from_slice(&post.created_at.0.to_le_bytes());
    match post.expires_at {
        Some(ts) => {
            out.push(1);
            out.extend_from_slice(&ts.0.to_le_bytes());
        }
        None => out.push(0),
    }
    Ok(())
}

fn valid_body(body: &str) -> Result<(), ProtocolError> {
    validate_body(body).map_err(|_| ProtocolError::Invalid)
}

pub fn encode(frame: &Frame) -> Result<Vec<u8>, ProtocolError> {
    let mut payload = Vec::new();
    match &frame.message {
        Message::CreatePostRequest(m) => {
            valid_body(&m.body)?;
            payload.extend_from_slice(m.actor_id.0.as_bytes());
            payload.extend_from_slice(&m.coordinate.lat_e7.to_le_bytes());
            payload.extend_from_slice(&m.coordinate.lon_e7.to_le_bytes());
            payload.push(m.kind as u8);
            payload.extend_from_slice(&(m.body.len() as u16).to_le_bytes());
            payload.extend_from_slice(m.body.as_bytes());
        }
        Message::CreatePostV2Request(m) => {
            valid_body(&m.body)?;
            payload.extend_from_slice(m.actor_id.0.as_bytes());
            payload.extend_from_slice(m.client_post_id.0.as_bytes());
            payload.extend_from_slice(&m.coordinate.lat_e7.to_le_bytes());
            payload.extend_from_slice(&m.coordinate.lon_e7.to_le_bytes());
            payload.push(m.kind as u8);
            payload.extend_from_slice(&(m.body.len() as u16).to_le_bytes());
            payload.extend_from_slice(m.body.as_bytes());
        }
        Message::CreatePostResponse(m) | Message::CreatePostV2Response(m) => {
            valid_cell(m.cell_id)?;
            payload.extend_from_slice(m.post_id.0.as_bytes());
            payload.extend_from_slice(&m.cell_id.0.to_le_bytes());
            payload.extend_from_slice(&m.cell_revision.to_le_bytes());
            payload.extend_from_slice(&m.created_at.0.to_le_bytes());
        }
        Message::QueryCellsRequest(m) => {
            valid_cells(&m.cells)?;
            payload.push(m.cells.len() as u8);
            for cell in &m.cells {
                payload.extend_from_slice(&cell.cell_id.0.to_le_bytes());
                payload.extend_from_slice(&cell.known_revision.to_le_bytes());
            }
        }
        Message::QueryCellsResponse(m) => {
            if m.cells.len() > MAX_QUERY_CELLS {
                return Err(ProtocolError::TooLarge);
            }
            payload.push(m.cells.len() as u8);
            let mut seen = std::collections::HashSet::new();
            for cell in &m.cells {
                let id = match cell {
                    CellResult::Unchanged { cell_id, .. }
                    | CellResult::Snapshot { cell_id, .. } => *cell_id,
                };
                valid_cell(id)?;
                if !seen.insert(id) {
                    return Err(ProtocolError::Invalid);
                }
                match cell {
                    CellResult::Unchanged { cell_id, revision } => {
                        payload.extend_from_slice(&cell_id.0.to_le_bytes());
                        payload.extend_from_slice(&revision.to_le_bytes());
                        payload.push(0);
                    }
                    CellResult::Snapshot {
                        cell_id,
                        revision,
                        posts,
                    } => {
                        if posts.len() > u16::MAX as usize {
                            return Err(ProtocolError::TooLarge);
                        }
                        payload.extend_from_slice(&cell_id.0.to_le_bytes());
                        payload.extend_from_slice(&revision.to_le_bytes());
                        payload.push(1);
                        payload.extend_from_slice(&(posts.len() as u16).to_le_bytes());
                        for post in posts {
                            if post.cell_id != *cell_id {
                                return Err(ProtocolError::Invalid);
                            }
                            push_post(&mut payload, post)?;
                        }
                    }
                }
            }
        }
        Message::ErrorResponse(m) => payload.extend_from_slice(&(m.code as u16).to_le_bytes()),
    }
    if payload.len() > MAX_PAYLOAD_BYTES {
        return Err(ProtocolError::TooLarge);
    }
    let mut out = Vec::with_capacity(HEADER_BYTES + payload.len());
    out.extend_from_slice(b"MPPA");
    out.push(1);
    out.push(frame.message.kind());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(&frame.request_id.to_le_bytes());
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(&payload);
    Ok(out)
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], ProtocolError> {
        let end = self.pos.checked_add(n).ok_or(ProtocolError::TooLarge)?;
        let part = self
            .bytes
            .get(self.pos..end)
            .ok_or(ProtocolError::Truncated)?;
        self.pos = end;
        Ok(part)
    }
    fn u8(&mut self) -> Result<u8, ProtocolError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, ProtocolError> {
        Ok(u16::from_le_bytes(
            self.take(2)?
                .try_into()
                .map_err(|_| ProtocolError::Invalid)?,
        ))
    }
    fn u32(&mut self) -> Result<u32, ProtocolError> {
        Ok(u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| ProtocolError::Invalid)?,
        ))
    }
    fn u64(&mut self) -> Result<u64, ProtocolError> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| ProtocolError::Invalid)?,
        ))
    }
    fn i32(&mut self) -> Result<i32, ProtocolError> {
        Ok(i32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| ProtocolError::Invalid)?,
        ))
    }
    fn i64(&mut self) -> Result<i64, ProtocolError> {
        Ok(i64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| ProtocolError::Invalid)?,
        ))
    }
    fn uuid(&mut self) -> Result<Uuid, ProtocolError> {
        Uuid::from_slice(self.take(16)?).map_err(|_| ProtocolError::Invalid)
    }
    fn cell(&mut self) -> Result<CellId, ProtocolError> {
        let id = CellId(self.u64()?);
        valid_cell(id)?;
        Ok(id)
    }
    fn string(&mut self) -> Result<String, ProtocolError> {
        let len = self.u16()? as usize;
        let body = std::str::from_utf8(self.take(len)?).map_err(|_| ProtocolError::Invalid)?;
        valid_body(body)?;
        Ok(body.to_owned())
    }
    fn done(&self) -> Result<(), ProtocolError> {
        if self.pos == self.bytes.len() {
            Ok(())
        } else {
            Err(ProtocolError::Invalid)
        }
    }
}

pub fn decode(bytes: &[u8]) -> Result<Frame, ProtocolError> {
    if bytes.len() < HEADER_BYTES {
        return Err(ProtocolError::Truncated);
    }
    let mut header = Reader::new(bytes);
    if header.take(4)? != b"MPPA" {
        return Err(ProtocolError::Invalid);
    }
    if header.u8()? != 1 {
        return Err(ProtocolError::UnsupportedVersion);
    }
    let kind = header.u8()?;
    if header.u16()? != 0 {
        return Err(ProtocolError::Invalid);
    }
    let request_id = header.u32()?;
    let len = header.u32()? as usize;
    if len > MAX_PAYLOAD_BYTES {
        return Err(ProtocolError::TooLarge);
    }
    if bytes.len() != HEADER_BYTES + len {
        return Err(ProtocolError::Invalid);
    }
    let mut r = Reader::new(&bytes[HEADER_BYTES..]);
    let message = match kind {
        1 => Message::CreatePostRequest(CreatePostRequest {
            actor_id: ActorId(r.uuid()?),
            coordinate: Coordinate::new(r.i32()?, r.i32()?).map_err(|_| ProtocolError::Invalid)?,
            kind: PostKind::try_from(r.u8()?).map_err(|_| ProtocolError::Invalid)?,
            body: r.string()?,
        }),
        2 | 7 => {
            let response = CreatePostResponse {
                post_id: PostId(r.uuid()?),
                cell_id: r.cell()?,
                cell_revision: r.u64()?,
                created_at: Timestamp(r.i64()?),
            };
            if kind == 2 {
                Message::CreatePostResponse(response)
            } else {
                Message::CreatePostV2Response(response)
            }
        }
        3 => {
            let count = r.u8()? as usize;
            if count == 0 || count > MAX_QUERY_CELLS {
                return Err(ProtocolError::TooLarge);
            }
            let mut cells = Vec::with_capacity(count);
            for _ in 0..count {
                cells.push(CellQuery {
                    cell_id: r.cell()?,
                    known_revision: r.u64()?,
                });
            }
            valid_cells(&cells)?;
            Message::QueryCellsRequest(QueryCellsRequest { cells })
        }
        4 => {
            let count = r.u8()? as usize;
            if count > MAX_QUERY_CELLS {
                return Err(ProtocolError::TooLarge);
            }
            let mut cells = Vec::with_capacity(count);
            let mut seen = std::collections::HashSet::new();
            for _ in 0..count {
                let cell_id = r.cell()?;
                if !seen.insert(cell_id) {
                    return Err(ProtocolError::Invalid);
                }
                let revision = r.u64()?;
                let status = r.u8()?;
                let cell = match status {
                    0 => CellResult::Unchanged { cell_id, revision },
                    1 => {
                        let count = r.u16()? as usize;
                        let mut posts = Vec::with_capacity(count);
                        for _ in 0..count {
                            let id = PostId(r.uuid()?);
                            let actor_id = ActorId(r.uuid()?);
                            let coordinate = Coordinate::new(r.i32()?, r.i32()?)
                                .map_err(|_| ProtocolError::Invalid)?;
                            if coordinate_to_cell(coordinate).map_err(|_| ProtocolError::Invalid)?
                                != cell_id
                            {
                                return Err(ProtocolError::Invalid);
                            }
                            let kind =
                                PostKind::try_from(r.u8()?).map_err(|_| ProtocolError::Invalid)?;
                            let body = r.string()?;
                            let created_at = Timestamp(r.i64()?);
                            let expires_at = match r.u8()? {
                                0 => None,
                                1 => Some(Timestamp(r.i64()?)),
                                _ => return Err(ProtocolError::Invalid),
                            };
                            posts.push(Post {
                                id,
                                actor_id,
                                coordinate,
                                cell_id,
                                kind,
                                body,
                                created_at,
                                expires_at,
                            });
                        }
                        CellResult::Snapshot {
                            cell_id,
                            revision,
                            posts,
                        }
                    }
                    _ => return Err(ProtocolError::Invalid),
                };
                cells.push(cell);
            }
            Message::QueryCellsResponse(QueryCellsResponse { cells })
        }
        5 => {
            let code = match r.u16()? {
                1 => ErrorCode::InvalidRequest,
                2 => ErrorCode::TooLarge,
                3 => ErrorCode::Internal,
                4 => ErrorCode::Conflict,
                _ => return Err(ProtocolError::Invalid),
            };
            Message::ErrorResponse(ErrorResponse { code })
        }
        6 => Message::CreatePostV2Request(CreatePostV2Request {
            actor_id: ActorId(r.uuid()?),
            client_post_id: ClientPostId(r.uuid()?),
            coordinate: Coordinate::new(r.i32()?, r.i32()?).map_err(|_| ProtocolError::Invalid)?,
            kind: PostKind::try_from(r.u8()?).map_err(|_| ProtocolError::Invalid)?,
            body: r.string()?,
        }),
        _ => return Err(ProtocolError::Invalid),
    };
    r.done()?;
    Ok(Frame {
        request_id,
        message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Vec<Frame> {
        let cell =
            mappa_spatial::coordinate_to_cell(Coordinate::new(375_665_000, 1_269_780_000).unwrap())
                .unwrap();
        let id = Uuid::new_v4();
        let post = Post {
            id: PostId(id),
            actor_id: ActorId(id),
            coordinate: Coordinate::new(375_665_000, 1_269_780_000).unwrap(),
            cell_id: cell,
            kind: PostKind::General,
            body: "안녕".into(),
            created_at: Timestamp(123),
            expires_at: None,
        };
        vec![
            Frame {
                request_id: 1,
                message: Message::CreatePostRequest(CreatePostRequest {
                    actor_id: ActorId(id),
                    coordinate: post.coordinate,
                    kind: PostKind::General,
                    body: post.body.clone(),
                }),
            },
            Frame {
                request_id: 2,
                message: Message::CreatePostResponse(CreatePostResponse {
                    post_id: post.id,
                    cell_id: cell,
                    cell_revision: 1,
                    created_at: Timestamp(123),
                }),
            },
            Frame {
                request_id: 6,
                message: Message::CreatePostV2Request(CreatePostV2Request {
                    actor_id: ActorId(id),
                    client_post_id: ClientPostId(Uuid::new_v4()),
                    coordinate: post.coordinate,
                    kind: PostKind::General,
                    body: post.body.clone(),
                }),
            },
            Frame {
                request_id: 7,
                message: Message::CreatePostV2Response(CreatePostResponse {
                    post_id: post.id,
                    cell_id: cell,
                    cell_revision: 1,
                    created_at: Timestamp(123),
                }),
            },
            Frame {
                request_id: 3,
                message: Message::QueryCellsRequest(QueryCellsRequest {
                    cells: vec![CellQuery {
                        cell_id: cell,
                        known_revision: 0,
                    }],
                }),
            },
            Frame {
                request_id: 4,
                message: Message::QueryCellsResponse(QueryCellsResponse {
                    cells: vec![
                        CellResult::Snapshot {
                            cell_id: cell,
                            revision: 1,
                            posts: vec![post],
                        },
                        CellResult::Unchanged {
                            cell_id: mappa_spatial::xy_to_cell(14, 0, 0).unwrap(),
                            revision: 0,
                        },
                    ],
                }),
            },
            Frame {
                request_id: 5,
                message: Message::ErrorResponse(ErrorResponse {
                    code: ErrorCode::InvalidRequest,
                }),
            },
        ]
    }
    #[test]
    fn all_messages_roundtrip() {
        for frame in sample() {
            assert_eq!(decode(&encode(&frame).unwrap()).unwrap(), frame);
        }
    }
    #[test]
    fn v1_create_golden_bytes_are_unchanged() {
        let frame = Frame {
            request_id: 1,
            message: Message::CreatePostRequest(CreatePostRequest {
                actor_id: ActorId(Uuid::nil()),
                coordinate: Coordinate::new(0, 0).unwrap(),
                kind: PostKind::General,
                body: "a".into(),
            }),
        };
        let mut expected = b"MPPA".to_vec();
        expected.extend_from_slice(&[1, 1, 0, 0, 1, 0, 0, 0, 28, 0, 0, 0]);
        expected.extend_from_slice(&[0; 16]);
        expected.extend_from_slice(&[0; 8]);
        expected.extend_from_slice(&[1, 1, 0, b'a']);
        assert_eq!(encode(&frame).unwrap(), expected);
        assert_eq!(decode(&expected).unwrap(), frame);
    }
    #[test]
    fn corrupt_frames_never_panic() {
        let mut good = encode(&sample()[0]).unwrap();
        for n in 0..good.len() {
            assert!(decode(&good[..n]).is_err());
        }
        good[0] = 0;
        assert!(decode(&good).is_err());
        good[0] = b'M';
        good[4] = 2;
        assert_eq!(decode(&good), Err(ProtocolError::UnsupportedVersion));
        good[4] = 1;
        good[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(decode(&good), Err(ProtocolError::TooLarge));
        let mut bad_coordinate = encode(&sample()[0]).unwrap();
        bad_coordinate[32..36].copy_from_slice(&i32::MAX.to_le_bytes());
        assert_eq!(decode(&bad_coordinate), Err(ProtocolError::Invalid));
        let mut bad_kind = encode(&sample()[0]).unwrap();
        bad_kind[40] = 255;
        assert_eq!(decode(&bad_kind), Err(ProtocolError::Invalid));
        let mut bad_utf8 = encode(&sample()[0]).unwrap();
        bad_utf8[43] = 255;
        assert_eq!(decode(&bad_utf8), Err(ProtocolError::Invalid));
        let mut duplicate = encode(&sample()[4]).unwrap();
        let entry = duplicate[17..33].to_vec();
        duplicate[16] = 2;
        duplicate[12..16].copy_from_slice(&33_u32.to_le_bytes());
        duplicate.extend_from_slice(&entry);
        assert_eq!(decode(&duplicate), Err(ProtocolError::Invalid));
        for n in 0..1000_u32 {
            let mut bytes = n.to_le_bytes().repeat(10);
            bytes.truncate((n as usize) % 40);
            let _ = decode(&bytes);
        }
    }
}
