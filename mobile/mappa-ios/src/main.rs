#[cfg(not(target_os = "ios"))]
fn main() {
    eprintln!("mappa-ios runs on iOS only");
}

mod ui {
    slint::slint! {
        export component MappaWindow inherits Window {
            in-out property <string> location_text: "위치 권한과 현재 위치를 확인합니다";
            in-out property <string> accuracy_text: "-";
            in-out property <string> cell_text: "-";
            in-out property <string> status_text: "시작 중";
            in-out property <string> draft: "";
            in-out property <bool> posting: false;
            in-out property <bool> connected: false;
            in-out property <bool> location_ready: false;
            in-out property <string> server_url: "";
            callback connect(string);
            callback submit(string);
            callback refresh_location();

            title: "MAPPA";
            width: 360px;
            height: 640px;
            VerticalLayout {
                padding: 20px;
                spacing: 12px;
                Text { text: "MAPPA"; font-size: 28px; }
                Text { text: "개발 HTTPS 서버 주소"; }
                Rectangle {
                    height: 44px;
                    border-color: #64748b;
                    border-width: 1px;
                    server_input := TextInput { width: parent.width; height: parent.height; text <=> root.server_url; }
                }
                Rectangle {
                    height: 44px;
                    background: root.posting ? #94a3b8 : #1d4ed8;
                    Text { text: root.connected ? "서버 주소 변경" : "서버 연결"; color: white; horizontal-alignment: center; vertical-alignment: center; }
                    TouchArea { enabled: !root.posting; clicked => { root.connect(server_input.text); } }
                }
                Text { text: "위치: " + root.location_text; wrap: word-wrap; }
                Text { text: "정확도: " + root.accuracy_text; }
                Text { text: "Cell: " + root.cell_text; wrap: word-wrap; }
                Rectangle {
                    height: 44px;
                    background: #dbeafe;
                    Text { text: "현재 위치 갱신"; horizontal-alignment: center; vertical-alignment: center; }
                    TouchArea { enabled: root.connected && !root.posting; clicked => { root.refresh_location(); } }
                }
                Text { text: "현재 위치에 남길 문장"; }
                Rectangle {
                    height: 80px;
                    border-color: #64748b;
                    border-width: 1px;
                    input := TextInput { width: parent.width; height: parent.height; text <=> root.draft; }
                }
                Rectangle {
                    height: 44px;
                    background: root.posting ? #94a3b8 : #0f766e;
                    Text { text: root.posting ? "게시 중" : "현재 위치에 게시"; color: white; horizontal-alignment: center; vertical-alignment: center; }
                    TouchArea { enabled: root.connected && root.location_ready && !root.posting; clicked => { root.submit(input.text); } }
                }
                Text { text: "상태: " + root.status_text; wrap: word-wrap; }
            }
        }
    }
}

#[cfg(target_os = "ios")]
mod ios {
    use crate::ui::MappaWindow;
    use mappa_apple::{AppleActorStore, AppleLocationProvider};
    use mappa_client_core::{CAMERA_IDLE_DEBOUNCE_MS, LOCAL_DETAIL_MAX_VIEWPORT_METERS};
    use mappa_client_runtime::{ClientRuntime, RuntimeError};
    use mappa_client_transport::HttpTransport;
    use mappa_device::{DeviceError, LocationStatus, load_or_create_actor, posting_coordinate};
    use mappa_spatial::coordinate_to_cell;
    use slint::{ComponentHandle, SharedString, Weak};
    use std::sync::mpsc::{self, Receiver};
    use std::time::{SystemTime, UNIX_EPOCH};

    enum Command {
        Connect(String),
        Post(String),
        RefreshLocation,
    }

    fn show_location(ui: &Weak<MappaWindow>, status: LocationStatus) {
        let valid = now_ms()
            .and_then(|now| posting_coordinate(status, now).ok())
            .is_some_and(|coordinate| coordinate_to_cell(coordinate).is_ok());
        let (location, accuracy, cell) = match status {
            LocationStatus::Ready(fix) => (
                if valid {
                    "준비됨".to_owned()
                } else {
                    match now_ms().map(|now| posting_coordinate(status, now)) {
                        Some(Err(DeviceError::StaleLocation)) => {
                            "위치가 오래됨. 다시 갱신해 주세요".to_owned()
                        }
                        Some(Err(DeviceError::InaccurateLocation)) => {
                            "위치 정확도가 부족함. 다시 갱신해 주세요".to_owned()
                        }
                        _ => "게시 가능한 위치가 아님".to_owned(),
                    }
                },
                format!("{:.0} m", fix.horizontal_accuracy_m),
                coordinate_to_cell(fix.coordinate)
                    .map_or_else(|_| "범위 밖".to_owned(), |cell| cell.0.to_string()),
            ),
            LocationStatus::Denied => (
                "권한 거부됨. iOS 설정에서 허용해 주세요".to_owned(),
                "-".to_owned(),
                "-".to_owned(),
            ),
            LocationStatus::Restricted => (
                "기기 설정으로 위치가 제한됨".to_owned(),
                "-".to_owned(),
                "-".to_owned(),
            ),
            LocationStatus::Unavailable => (
                "현재 위치를 사용할 수 없음".to_owned(),
                "-".to_owned(),
                "-".to_owned(),
            ),
            LocationStatus::Error => ("위치 오류".to_owned(), "-".to_owned(), "-".to_owned()),
            _ => (
                "위치를 확인하는 중".to_owned(),
                "-".to_owned(),
                "-".to_owned(),
            ),
        };
        let _ = ui.upgrade_in_event_loop(move |window| {
            window.set_location_text(location.into());
            window.set_accuracy_text(accuracy.into());
            window.set_cell_text(cell.into());
            window.set_location_ready(valid);
        });
    }

    fn show_status(ui: &Weak<MappaWindow>, message: &'static str, posting: bool) {
        let _ = ui.upgrade_in_event_loop(move |window| {
            window.set_status_text(SharedString::from(message));
            window.set_posting(posting);
        });
    }

    fn restore_server_url(ui: &Weak<MappaWindow>, active_url: Option<&str>) {
        let url = active_url.unwrap_or_default().to_owned();
        let _ = ui.upgrade_in_event_loop(move |window| window.set_server_url(url.into()));
    }

    fn now_ms() -> Option<i64> {
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_millis();
        i64::try_from(ms).ok()
    }

    fn post_error(error: &RuntimeError) -> &'static str {
        match error {
            RuntimeError::Device(_) => "위치를 확인할 수 없습니다. 다시 갱신해 주세요",
            RuntimeError::Domain(_) => "문장은 UTF-8 1~8192바이트여야 합니다",
            RuntimeError::Spatial(_) => "이 지역은 아직 지원되지 않습니다",
            RuntimeError::Conflict => "이 게시 요청의 내용이 이전 전송과 다릅니다",
            RuntimeError::Clock => "기기 시간이 올바르지 않습니다",
            RuntimeError::Transport(_) => {
                "서버 연결에 실패했습니다. 같은 문장으로 다시 시도해 주세요"
            }
            _ => "응답을 확인할 수 없습니다. 다시 시도해 주세요",
        }
    }

    async fn worker(receiver: Receiver<Command>, ui: Weak<MappaWindow>) {
        let mut runtime: Option<ClientRuntime<AppleLocationProvider, HttpTransport>> = None;
        let mut active_url: Option<String> = None;
        let mut store = AppleActorStore;
        if load_or_create_actor(&mut store).is_ok() {
            show_status(&ui, "기기 ID 준비됨. 개발 서버 주소를 입력하세요", false);
        } else {
            show_status(&ui, "기기 ID 저장소를 열 수 없습니다", false);
        }

        while let Ok(command) = receiver.recv() {
            match command {
                Command::Connect(url) => {
                    show_status(&ui, "서버 주소 확인 중", true);
                    let requested_url = url.trim();
                    let transport = match HttpTransport::new(requested_url) {
                        Ok(transport) => transport,
                        Err(_) => {
                            restore_server_url(&ui, active_url.as_deref());
                            show_status(&ui, "HTTPS 서버 주소를 확인해 주세요", false);
                            continue;
                        }
                    };
                    if transport.check_health().await.is_err() {
                        restore_server_url(&ui, active_url.as_deref());
                        show_status(&ui, "서버에 연결할 수 없습니다", false);
                        continue;
                    }
                    if let Some(existing) = runtime.as_mut() {
                        existing.replace_transport(transport);
                    } else {
                        let mut store = AppleActorStore;
                        runtime = match ClientRuntime::new(
                            &mut store,
                            AppleLocationProvider,
                            transport,
                        ) {
                            Ok(runtime) => Some(runtime),
                            Err(_) => {
                                show_status(&ui, "기기 ID 저장소를 열 수 없습니다", false);
                                continue;
                            }
                        };
                    }
                    active_url = Some(requested_url.to_owned());
                    let _ = ui.upgrade_in_event_loop(|window| window.set_connected(true));
                    let Some(active) = runtime.as_mut() else {
                        continue;
                    };
                    show_status(&ui, "위치를 확인하는 중", true);
                    let status = active.refresh_location().await;
                    show_location(&ui, status);
                    show_status(&ui, "문장을 입력할 수 있습니다", false);
                }
                Command::RefreshLocation => {
                    let Some(active) = runtime.as_mut() else {
                        continue;
                    };
                    show_status(&ui, "위치를 확인하는 중", false);
                    let status = active.refresh_location().await;
                    show_location(&ui, status);
                    show_status(&ui, "위치 확인 완료", false);
                }
                Command::Post(body) => {
                    let Some(runtime) = runtime.as_mut() else {
                        continue;
                    };
                    show_status(&ui, "게시 중", true);
                    let result = runtime
                        .post_with_clock(&body, || now_ms().ok_or(RuntimeError::Clock))
                        .await;
                    show_location(&ui, runtime.location_status());
                    match result {
                        Ok(created) => {
                            let coordinate = match runtime.location_status() {
                                LocationStatus::Ready(fix) => fix.coordinate,
                                _ => {
                                    show_status(&ui, "게시됨. 위치 재조회 필요", false);
                                    continue;
                                }
                            };
                            let Some(idle_at) = now_ms().and_then(|v| u64::try_from(v).ok()) else {
                                show_status(&ui, "게시됨. 기기 시간 오류로 조회 실패", false);
                                runtime.acknowledge_post();
                                continue;
                            };
                            runtime.camera_idle(
                                coordinate,
                                LOCAL_DETAIL_MAX_VIEWPORT_METERS,
                                idle_at,
                            );
                            let query_result = runtime
                                .refresh_visible_cells(
                                    idle_at.saturating_add(CAMERA_IDLE_DEBOUNCE_MS),
                                )
                                .await;
                            let confirmed = query_result.is_ok()
                                && runtime.core().cache().get(&created.cell_id).is_some_and(
                                    |entry| {
                                        entry.posts.iter().any(|post| post.id == created.post_id)
                                    },
                                );
                            show_status(
                                &ui,
                                if confirmed {
                                    "게시 및 같은 지역 조회 확인"
                                } else {
                                    "게시됨. 조회 확인 실패"
                                },
                                false,
                            );
                            let _ = ui.upgrade_in_event_loop(|window| {
                                window.set_draft(SharedString::default())
                            });
                            runtime.acknowledge_post();
                        }
                        Err(error) => show_status(&ui, post_error(&error), false),
                    }
                }
            }
        }
    }

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let ui = MappaWindow::new()?;
        if let Some(url) = option_env!("MAPPA_API_BASE_URL") {
            ui.set_server_url(url.into());
        }
        let (sender, receiver) = mpsc::channel();
        let connect_sender = sender.clone();
        let connect_weak = ui.as_weak();
        ui.on_connect(move |url| {
            if connect_sender
                .send(Command::Connect(url.to_string()))
                .is_ok()
            {
                if let Some(window) = connect_weak.upgrade() {
                    window.set_posting(true);
                }
            }
        });
        let post_sender = sender.clone();
        let weak = ui.as_weak();
        ui.on_submit(move |body| {
            if post_sender.send(Command::Post(body.to_string())).is_ok() {
                if let Some(window) = weak.upgrade() {
                    window.set_posting(true);
                }
            }
        });
        ui.on_refresh_location(move || {
            let _ = sender.send(Command::RefreshLocation);
        });
        let worker_ui = ui.as_weak();
        std::thread::spawn(move || {
            if let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                runtime.block_on(worker(receiver, worker_ui));
            } else {
                show_status(&worker_ui, "비동기 실행기를 시작할 수 없습니다", false);
            }
        });
        ui.run()?;
        Ok(())
    }
}

#[cfg(target_os = "ios")]
fn main() {
    if let Err(error) = ios::run() {
        eprintln!("Mappa UI initialization failed: {error}");
    }
}
