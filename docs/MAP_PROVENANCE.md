# 출처 이력 경계

각 도로 중심선·도로면·읍면동 지명·ESA 영구수면·수목 피복의 GeoDB provenance에는 canonical ID, 원천 ID, 원본 `gid`/`ADM_CD` 또는 polygon feature 해시, 원천 버전, adapter version, manifest에 잠긴 GeoJSON 입력 feature SHA-256이 있다. ESA의 crop TIFF와 polygonized GeoJSON은 모두 manifest SHA-256으로 잠근다. 타일에는 provenance 필드가 없고 실제 geometry·지명만 들어간다. SGIS 지명 위치는 원본 행정경계 중 proof bbox 안의 부분에서 Rust `interior_point`로 결정했다. 빌드 로그와 manifest로 입력 파일까지 역추적한다. 같은 manifest·입력·현재 builder로 재생성한 5개 입력 GeoDB와 PMTiles는 각각 SHA-256이 일치했다.

현재는 나주시 도로 2개 레이어, 별도 SGIS 행정 지명, ESA COG의 두 분류 레이어가 있다. 서로 다른 종류의 객체라 동일 객체 matching/fusion은 발생하지 않았으며 `match_confidence`와 `fusion_rule_version`이 없다. 업데이트 시 원본 객체 ID 유지와 revision 연결도 아직 검증하지 않았다. 같은 종류의 다원천 융합을 시작하기 전 이 항목을 추가하고 충돌을 별도 QA로 보내야 한다.
