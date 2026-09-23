use mappa_device::{
    ActorStore, DeviceError, LocationFix, LocationProvider, LocationStatus, degrees_to_coordinate,
};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send};
use objc2_core_location::{
    CLAuthorizationStatus, CLLocation, CLLocationManager, CLLocationManagerDelegate,
    kCLLocationAccuracyNearestTenMeters,
};
use objc2_foundation::{MainThreadMarker, NSArray, NSError, NSObject, NSObjectProtocol};
use security_framework::passwords::{
    PasswordOptions, generic_password, set_generic_password_options,
};
use security_framework_sys::base::errSecItemNotFound;
use std::{cell::RefCell, time::Duration};
use tokio::sync::oneshot;

const SERVICE: &str = "com.seoyd.mappa.actor";
const ACCOUNT: &str = "device-local-v1";

pub struct AppleActorStore;
impl ActorStore for AppleActorStore {
    fn load(&mut self) -> Result<Option<Vec<u8>>, DeviceError> {
        let mut options = PasswordOptions::new_generic_password(SERVICE, ACCOUNT);
        options.set_access_synchronized(Some(false));
        match generic_password(options) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.code() == errSecItemNotFound => Ok(None),
            Err(error) => {
                eprintln!("Keychain load failed: OSStatus {}", error.code());
                Err(DeviceError::IdentityStore)
            }
        }
    }
    fn save(&mut self, bytes: &[u8; 16]) -> Result<(), DeviceError> {
        let mut options = PasswordOptions::new_generic_password(SERVICE, ACCOUNT);
        options.set_access_synchronized(Some(false));
        set_generic_password_options(bytes, options).map_err(|error| {
            eprintln!("Keychain save failed: OSStatus {}", error.code());
            DeviceError::IdentityStore
        })
    }
}

struct DelegateIvars {
    pending: RefCell<Option<oneshot::Sender<LocationStatus>>>,
}
define_class!(
    // SAFETY: NSObject has no subclass requirements; callbacks run on the main thread.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = DelegateIvars]
    struct MappaLocationDelegate;
    unsafe impl NSObjectProtocol for MappaLocationDelegate {}
    unsafe impl CLLocationManagerDelegate for MappaLocationDelegate {
        #[unsafe(method(locationManager:didUpdateLocations:))]
        unsafe fn location_manager_did_update_locations(
            &self,
            _manager: &CLLocationManager,
            locations: &NSArray<CLLocation>,
        ) {
            let status = match locations.lastObject() {
                Some(location) => {
                    // SAFETY: CoreLocation supplies a live CLLocation object in this delegate callback.
                    let raw = unsafe { location.coordinate() };
                    let coordinate = degrees_to_coordinate(raw.latitude, raw.longitude);
                    let accuracy = unsafe { location.horizontalAccuracy() };
                    let seconds = unsafe { location.timestamp() }.timeIntervalSince1970();
                    match coordinate {
                        Ok(coordinate)
                            if seconds.is_finite()
                                && seconds >= 0.0
                                && seconds * 1000.0 <= i64::MAX as f64 =>
                        {
                            LocationStatus::Ready(LocationFix {
                                coordinate,
                                horizontal_accuracy_m: accuracy,
                                captured_at_ms: (seconds * 1000.0).round() as i64,
                            })
                        }
                        _ => LocationStatus::Error,
                    }
                }
                None => LocationStatus::Unavailable,
            };
            self.finish(status);
        }
        #[unsafe(method(locationManager:didFailWithError:))]
        unsafe fn location_manager_did_fail_with_error(
            &self,
            _manager: &CLLocationManager,
            _error: &NSError,
        ) {
            self.finish(LocationStatus::Unavailable);
        }
        #[unsafe(method(locationManagerDidChangeAuthorization:))]
        unsafe fn location_manager_did_change_authorization(&self, manager: &CLLocationManager) {
            // SAFETY: CoreLocation owns the manager and calls on the creating thread.
            match unsafe { manager.authorizationStatus() } {
                CLAuthorizationStatus::AuthorizedWhenInUse
                | CLAuthorizationStatus::AuthorizedAlways => unsafe { manager.requestLocation() },
                CLAuthorizationStatus::Denied => self.finish(LocationStatus::Denied),
                CLAuthorizationStatus::Restricted => self.finish(LocationStatus::Restricted),
                _ => {}
            }
        }
    }
);
impl MappaLocationDelegate {
    fn new(main_thread: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(main_thread).set_ivars(DelegateIvars {
            pending: RefCell::new(None),
        });
        // SAFETY: NSObject's designated initializer is valid for this subclass.
        unsafe { msg_send![super(this), init] }
    }
    fn finish(&self, status: LocationStatus) {
        if let Some(sender) = self.ivars().pending.borrow_mut().take() {
            let _ = sender.send(status);
        }
    }
}

struct Controller {
    manager: Retained<CLLocationManager>,
    delegate: Retained<MappaLocationDelegate>,
}
impl Controller {
    fn new() -> Option<Self> {
        let main_thread = MainThreadMarker::new()?;
        // SAFETY: The controller is constructed and retained on Slint's iOS main thread.
        let manager = unsafe { CLLocationManager::new() };
        let delegate = MappaLocationDelegate::new(main_thread);
        unsafe {
            manager.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
            manager.setDesiredAccuracy(kCLLocationAccuracyNearestTenMeters);
        }
        Some(Self { manager, delegate })
    }
    fn request(&self, sender: oneshot::Sender<LocationStatus>) {
        // SAFETY: CoreLocation manager methods run on its creating main thread.
        if !unsafe { CLLocationManager::locationServicesEnabled_class() } {
            let _ = sender.send(LocationStatus::Unavailable);
            return;
        }
        let mut pending = self.delegate.ivars().pending.borrow_mut();
        if pending.is_some() {
            let _ = sender.send(LocationStatus::Error);
            return;
        }
        *pending = Some(sender);
        drop(pending);
        match unsafe { self.manager.authorizationStatus() } {
            CLAuthorizationStatus::NotDetermined => unsafe {
                self.manager.requestWhenInUseAuthorization()
            },
            CLAuthorizationStatus::AuthorizedWhenInUse
            | CLAuthorizationStatus::AuthorizedAlways => unsafe { self.manager.requestLocation() },
            CLAuthorizationStatus::Denied => self.delegate.finish(LocationStatus::Denied),
            CLAuthorizationStatus::Restricted => self.delegate.finish(LocationStatus::Restricted),
            _ => self.delegate.finish(LocationStatus::Error),
        }
    }
    fn cancel(&self) {
        self.delegate.ivars().pending.borrow_mut().take();
        // SAFETY: This is the documented cancellation method for an outstanding location request.
        unsafe { self.manager.stopUpdatingLocation() };
    }
}
thread_local! { static CONTROLLER: RefCell<Option<Controller>> = const { RefCell::new(None) }; }

pub struct AppleLocationProvider;
impl LocationProvider for AppleLocationProvider {
    async fn request_location(&mut self) -> LocationStatus {
        let (sender, receiver) = oneshot::channel();
        if slint::invoke_from_event_loop(move || {
            CONTROLLER.with(|slot| {
                let mut controller = slot.borrow_mut();
                if controller.is_none() {
                    *controller = Controller::new();
                }
                if let Some(controller) = controller.as_ref() {
                    controller.request(sender);
                } else {
                    let _ = sender.send(LocationStatus::Error);
                }
            });
        })
        .is_err()
        {
            return LocationStatus::Error;
        }
        match tokio::time::timeout(Duration::from_secs(20), receiver).await {
            Ok(Ok(status)) => status,
            _ => {
                let _ = slint::invoke_from_event_loop(|| {
                    CONTROLLER.with(|slot| {
                        if let Some(controller) = slot.borrow().as_ref() {
                            controller.cancel();
                        }
                    })
                });
                LocationStatus::Unavailable
            }
        }
    }
}
