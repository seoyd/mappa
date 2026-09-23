use thiserror::Error;
use uuid::Uuid;

pub const MAX_BODY_BYTES: usize = 8192;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct PostId(pub Uuid);
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ActorId(pub Uuid);
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct CellId(pub u64);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Timestamp(pub i64); // Unix milliseconds

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Coordinate {
    pub lat_e7: i32,
    pub lon_e7: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PostKind {
    General = 1,
}

impl TryFrom<u8> for PostKind {
    type Error = DomainError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::General),
            _ => Err(DomainError::InvalidKind),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Post {
    pub id: PostId,
    pub actor_id: ActorId,
    pub coordinate: Coordinate,
    pub cell_id: CellId,
    pub kind: PostKind,
    pub body: String,
    pub created_at: Timestamp,
    pub expires_at: Option<Timestamp>,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum DomainError {
    #[error("coordinate outside WGS84 range")]
    InvalidCoordinate,
    #[error("body must contain non-whitespace text, no NUL, and at most 8192 UTF-8 bytes")]
    InvalidBody,
    #[error("unknown post kind")]
    InvalidKind,
}

impl Coordinate {
    pub fn new(lat_e7: i32, lon_e7: i32) -> Result<Self, DomainError> {
        if !(-900_000_000..=900_000_000).contains(&lat_e7)
            || !(-1_800_000_000..=1_800_000_000).contains(&lon_e7)
        {
            return Err(DomainError::InvalidCoordinate);
        }
        Ok(Self { lat_e7, lon_e7 })
    }
}

pub fn validate_body(body: &str) -> Result<(), DomainError> {
    if body.len() > MAX_BODY_BYTES || body.contains('\0') || body.trim().is_empty() {
        return Err(DomainError::InvalidBody);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coordinate_bounds() {
        assert!(Coordinate::new(900_000_000, -1_800_000_000).is_ok());
        assert!(Coordinate::new(900_000_001, 0).is_err());
        assert!(Coordinate::new(0, i32::MAX).is_err());
    }
    #[test]
    fn body_preserves_original() {
        assert!(validate_body("  ").is_err());
        assert!(validate_body("a\0b").is_err());
        assert!(validate_body(&"a".repeat(MAX_BODY_BYTES + 1)).is_err());
        assert!(validate_body(" 은어 그대로 ").is_ok());
    }
}
