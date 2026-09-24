# 출처 이력 경계

각 도로 중심선·도로면의 GeoDB provenance에는 canonical ID, 원천 ID, 원본 `gid`, 원천 날짜, adapter version, 원본 feature SHA-256이 있다. 타일에는 이 필드가 없고 도로 geometry만 들어간다. 빌드 로그와 manifest로 원본 파일까지 역추적한다. 같은 manifest·원본·현재 builder로 재생성한 GeoDB 및 PMTiles가 바이트 단위로 일치했다.

현재는 같은 나주시 공식 ZIP의 두 레이어뿐이므로 독립 원천 사이의 `match_confidence`와 `fusion_rule_version`이 없다. 업데이트 시 원본 객체 ID 유지와 revision 연결도 아직 검증하지 않았다. 다원천 융합을 시작하기 전 이 항목을 추가하고 충돌을 별도 QA로 보내야 한다.
