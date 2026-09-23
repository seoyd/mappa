# Decisions

- ADR-001: 콘텐츠 cell은 z14 Web Mercator 타일을 사용한다. 고정된 지역 조회 단위와 3x3 batch를 단순하게 구현할 수 있다. ±85.0511287°를 넘는 위도는 명시적으로 거부한다.
- ADR-002: canonical 좌표는 E7 정수다. wire와 DB 사이 표현을 정확히 유지한다. 타일 계산 과정에만 f64를 사용한다.
- ADR-003: 일반 조회는 cell_id index를 쓴다. PostGIS는 canonical 공간 컬럼과 향후 정밀 거리 질의용이다.
- ADR-004: revision이 같으면 unchanged, 다르면 완전 snapshot을 전송한다. delta 일관성 문제를 v0.1에서 피한다.
- ADR-005: 지도와 기기 위치 adapter는 client core 바깥에 둔다. v0.1은 UI 없이 좌표 입력부터 서버 저장·조회까지 검증한다.
- ADR-006: 사용자의 Rust-only 지시에 따라 앞으로 직접 작성하는 서버·클라이언트·UI·지도 처리·AI·라우팅 코드는 Rust로 통일한다. Web은 필요할 때 Rust/WASM으로 만든다. 기존 명세의 MapLibre UI 연결은 기술 선택을 확정한 것이 아니며 JavaScript/TypeScript UI를 도입하지 않는다. PostgreSQL/PostGIS의 SQL migration은 데이터베이스 스키마 정의이므로 유지한다.
