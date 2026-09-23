# Decisions

- ADR-001: 콘텐츠 cell은 z14 Web Mercator 타일을 사용한다. 고정된 지역 조회 단위와 3x3 batch를 단순하게 구현할 수 있다. ±85.0511287°를 넘는 위도는 명시적으로 거부한다.
- ADR-002: canonical 좌표는 E7 정수다. wire와 DB 사이 표현을 정확히 유지한다. 타일 계산 과정에만 f64를 사용한다.
- ADR-003: 일반 조회는 cell_id index를 쓴다. PostGIS는 canonical 공간 컬럼과 향후 정밀 거리 질의용이다.
- ADR-004: revision이 같으면 unchanged, 다르면 완전 snapshot을 전송한다. delta 일관성 문제를 v0.1에서 피한다.
- ADR-005: 지도와 기기 위치 adapter는 client core 바깥에 둔다. v0.1은 UI 없이 좌표 입력부터 서버 저장·조회까지 검증한다.
- ADR-006: 사용자의 Rust-only 지시에 따라 앞으로 직접 작성하는 서버·클라이언트·UI·지도 처리·AI·라우팅 코드는 Rust로 통일한다. Web은 필요할 때 Rust/WASM으로 만든다. 기존 명세의 MapLibre UI 연결은 기술 선택을 확정한 것이 아니며 JavaScript/TypeScript UI를 도입하지 않는다. PostgreSQL/PostGIS의 SQL migration은 데이터베이스 스키마 정의이므로 유지한다.
- ADR-007: v0.2는 iOS부터 실제 기기 경로를 연결한다. 진단/게시 shell만 구현하고 세계 지도와 Android adapter는 다음 단계로 미룬다.
- ADR-008: `LocationProvider`, `ActorStore`, `Transport`는 플랫폼 중립 Rust 계약으로 둔다. `ClientRuntime`은 CoreLocation/Keychain/HTTP 구현 타입을 알지 않는다.
- ADR-009: `ActorId`는 기기 로컬 익명 식별자이며 인증이나 위치 증명이 아니다. Keychain에 UUIDv4 원시 16바이트로 보관하고 iCloud 동기화를 켜지 않는다. 손상된 값은 조용히 교체하지 않는다.
- ADR-010: 게시 시 foreground one-shot 위치를 새로 요청한다. 허용 기준은 제품 초기 정책으로 30초 이내, 수평 정확도 100m 이내다. 오래되거나 부정확한 fix는 화면에 관찰 가능하지만 게시에는 사용하지 않는다. 항상 위치 추적/백그라운드 권한은 사용하지 않는다.
- ADR-011: v1 wire kinds 1~5를 유지하며 6/7을 idempotent create로 추가한다. `(actor_id, client_post_id)`의 DB unique index가 동시 요청의 최종 방어선이다. 첫 insert 때만 revision을 증가시키고 저장된 `create_revision`으로 원래 응답을 재구성한다. 같은 ID의 다른 내용은 HTTP 409다.
- ADR-012: 모바일 UI는 Rust 소스 내 `slint::slint!`로 작성한다. Slint의 iOS winit/Skia 백엔드를 사용한다. Xcodegen YAML/Info.plist는 앱 패키징 설정이며 앱 로직이 아니다. 앱 빌드와 기기 검증은 전체 Xcode 설치가 있는 환경에서 수행한다.
- ADR-013: 클라이언트가 전송하는 현재 위치는 정상 사용자의 UX 입력이다. 서버가 GPS spoofing 또는 기기 변조를 암호학적으로 탐지할 수 있다는 주장은 하지 않는다.
- ADR-014: v0.2.1 외부 네트워크 검증에는 Mac의 localhost Axum/PostGIS 앞에 HTTPS Cloudflare Quick Tunnel을 둔다. URL은 실행 때 앱 화면에 입력하고 연결 시 health를 확인한다. URL 변경에 재빌드는 필요하지 않으며 미확정 게시의 `ClientPostId`는 유지한다.
- ADR-015: Quick Tunnel은 공개·임시 개발 경로이며 운영 호스팅이나 인증 경계가 아니다. 인증 없는 API가 노출되므로 별도 일회성 DB와 테스트 글만 사용하고 테스트 직후 터널을 종료한다. 영구 서버/DB 이전은 이번 범위에서 하지 않는다.
- ADR-016: 향후 세계 basemap은 Mappa 클라이언트가 보유·렌더링하는 데이터를 전제로 한다. 외부 지도 API에 의존하지 않는다. PMTiles/decoder/renderer 선택은 v0.2.1 실기기 closure 후 v0.3 feasibility harness에서 판정하며 이 ADR이 구현 기술을 확정하지 않는다.
- ADR-017: Xcodegen의 Rust 빌드 단계는 매 빌드 실행한다. Cargo가 증분 재빌드 여부를 결정하며, Xcode가 출력 바이너리만 보고 Rust 소스 변경을 건너뛰면 오래된 앱을 배포할 수 있다.
- ADR-018: Keychain 접근은 정상적인 Apple 앱 서명과 앱 식별자 권한이 전제다. 프로젝트에서 entitlements를 생성하지만, 팀 서명이 없는 개발 환경의 Simulator에서 `-34018`을 확인했다. 임의 서명으로 보안 검사를 우회하지 않는다.
