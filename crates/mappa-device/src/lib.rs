use mappa_domain::{ActorId, Coordinate};
use thiserror::Error;
use uuid::{Uuid, Variant};

pub const MAX_POST_LOCATION_AGE_MS: i64 = 30_000;
pub const MAX_POST_HORIZONTAL_ACCURACY_M: f64 = 100.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocationFix {
    pub coordinate: Coordinate,
    pub horizontal_accuracy_m: f64,
    pub captured_at_ms: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LocationStatus {
    Unknown,
    PermissionRequired,
    RequestingPermission,
    Locating,
    Ready(LocationFix),
    Denied,
    Restricted,
    Unavailable,
    Error,
}

#[derive(Debug, Error, PartialEq)]
pub enum DeviceError {
    #[error("invalid degree coordinate")]
    InvalidCoordinate,
    #[error("location unavailable or permission denied")]
    LocationUnavailable,
    #[error("location is stale")]
    StaleLocation,
    #[error("location is not accurate enough")]
    InaccurateLocation,
    #[error("stored identity is corrupt")]
    CorruptIdentity,
    #[error("identity storage failed")]
    IdentityStore,
}

pub fn degrees_to_coordinate(lat: f64, lon: f64) -> Result<Coordinate, DeviceError> {
    if !lat.is_finite()
        || !lon.is_finite()
        || !(-90.0..=90.0).contains(&lat)
        || !(-180.0..=180.0).contains(&lon)
    {
        return Err(DeviceError::InvalidCoordinate);
    }
    Coordinate::new(
        (lat * 10_000_000.0).round() as i32,
        (lon * 10_000_000.0).round() as i32,
    )
    .map_err(|_| DeviceError::InvalidCoordinate)
}

pub fn posting_coordinate(status: LocationStatus, now_ms: i64) -> Result<Coordinate, DeviceError> {
    let LocationStatus::Ready(fix) = status else {
        return Err(DeviceError::LocationUnavailable);
    };
    if fix.captured_at_ms > now_ms
        || now_ms.saturating_sub(fix.captured_at_ms) > MAX_POST_LOCATION_AGE_MS
    {
        return Err(DeviceError::StaleLocation);
    }
    if !fix.horizontal_accuracy_m.is_finite()
        || fix.horizontal_accuracy_m < 0.0
        || fix.horizontal_accuracy_m > MAX_POST_HORIZONTAL_ACCURACY_M
    {
        return Err(DeviceError::InaccurateLocation);
    }
    Ok(fix.coordinate)
}

pub trait ActorStore {
    fn load(&mut self) -> Result<Option<Vec<u8>>, DeviceError>;
    fn save(&mut self, bytes: &[u8; 16]) -> Result<(), DeviceError>;
}

pub fn load_or_create_actor(store: &mut impl ActorStore) -> Result<ActorId, DeviceError> {
    if let Some(bytes) = store.load()? {
        let id = Uuid::from_slice(&bytes).map_err(|_| DeviceError::CorruptIdentity)?;
        if id.get_version_num() != 4 || id.get_variant() != Variant::RFC4122 {
            return Err(DeviceError::CorruptIdentity);
        }
        return Ok(ActorId(id));
    }
    let id = Uuid::new_v4();
    store.save(id.as_bytes())?;
    Ok(ActorId(id))
}

#[allow(async_fn_in_trait)] // Internal adapter contract; iOS callback futures need not be Send.
pub trait LocationProvider {
    async fn request_location(&mut self) -> LocationStatus;
}

#[cfg(test)]
mod tests {
    use super::*;
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
    #[test]
    fn actor_identity_persists_and_corruption_is_explicit() {
        let mut first = MemoryStore::default();
        let actor = load_or_create_actor(&mut first).unwrap();
        for _ in 0..100 {
            assert_eq!(load_or_create_actor(&mut first).unwrap(), actor);
        }
        let mut independent = MemoryStore::default();
        assert_ne!(load_or_create_actor(&mut independent).unwrap(), actor);
        first.0 = Some(vec![1, 2, 3]);
        assert_eq!(
            load_or_create_actor(&mut first),
            Err(DeviceError::CorruptIdentity)
        );
        first.0 = Some(vec![0; 16]);
        assert_eq!(
            load_or_create_actor(&mut first),
            Err(DeviceError::CorruptIdentity)
        );
    }
    #[test]
    fn degrees_and_posting_gate() {
        assert_eq!(
            degrees_to_coordinate(37.12345675, 126.0).unwrap().lat_e7,
            371_234_568
        );
        for value in [f64::NAN, f64::INFINITY, 91.0] {
            assert!(degrees_to_coordinate(value, 0.0).is_err());
        }
        let fix = LocationFix {
            coordinate: Coordinate::new(0, 0).unwrap(),
            horizontal_accuracy_m: 20.0,
            captured_at_ms: 1_000,
        };
        assert_eq!(
            posting_coordinate(LocationStatus::Ready(fix), 2_000).unwrap(),
            fix.coordinate
        );
        assert_eq!(
            posting_coordinate(LocationStatus::Ready(fix), 31_001),
            Err(DeviceError::StaleLocation)
        );
        assert_eq!(
            posting_coordinate(LocationStatus::Denied, 2_000),
            Err(DeviceError::LocationUnavailable)
        );
        assert_eq!(
            posting_coordinate(
                LocationStatus::Ready(LocationFix {
                    horizontal_accuracy_m: 101.0,
                    ..fix
                }),
                2_000
            ),
            Err(DeviceError::InaccurateLocation)
        );
        assert_eq!(
            posting_coordinate(
                LocationStatus::Ready(LocationFix {
                    horizontal_accuracy_m: -1.0,
                    ..fix
                }),
                2_000
            ),
            Err(DeviceError::InaccurateLocation)
        );
    }
}
