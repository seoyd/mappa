# Mappa 오프라인 세계지도 — 2026-09-24

## 방향과 원본 구분

기본 실행은 빈 화면 대신 실제 세계 지형을 보여준다. **지도 투영·타일 생성·렌더링·색과 글자 배치는 Mappa의 Rust 구현**이다. 대륙·해안·국경·도시 좌표는 Mappa가 측량한 것이 아니라 [Natural Earth의 퍼블릭 도메인 지도 데이터](https://www.naturalearthdata.com/about/terms-of-use/)다. 상용 지도의 화면이나 데이터베이스를 복제하지 않는다. Natural Earth의 공식 설명에 따르면 이 데이터는 수정과 상업적 사용이 허용된다. 위치 정확성과 최신성은 그 원본의 범위에 따른다.

지도 조회는 로컬 PMTiles 파일만 사용하며 지도 타일 서버나 외부 지도 API를 호출하지 않는다. 이것은 **운영 중 지도 타일 전송 비용이 없다는 뜻**이다. 지형 원본을 독점 소유한다는 뜻은 아니다.

## 현재 화면 단계

| 확대 단계 | 세계 범위 | 보이는 내용 | 실제 원본 수준 |
|---|---|---|---|
| z0–z4 | 전 세계 | 대륙·호수·국경·나라 이름 | Natural Earth 1:110m |
| z5–z7 | 전 세계 | 더 자세한 해안·호수·국경, z6부터 주요 도시 | Natural Earth 1:50m |
| z5–z7 | 동아시아 한정 | 더 자세한 해안·호수·국경·주요 도로·도시 | 기존 Natural Earth 1:10m 시안 |

동아시아 상세 영역에 완전히 들어가는 타일은 1:10m 파일을, 다른 세계 지역은 1:50m 파일을 읽는다. z7 이후 화면 확대는 가능하지만 새로운 도로·건물·역 정보가 생기지는 않는다. 화면의 m/px는 표시 축척이며 지리 위치 오차가 아니다. 전 세계 도로·주소·길찾기·건물 수준의 지도로 검증된 상태는 아니다.

## 확인된 진행 상황

| 항목 | 실제 결과 |
|---|---|
| 전 세계 중간 상세 빌드 | z5–z7, 타일 10,690개, 인코딩된 피처 23,518개, `world_50m.pmtiles` 1,608,226바이트 |
| 전 타일 감사 | 비어 있지 않은 타일 10,690개 전부 해독, 오류 0개. 지형 16,634개, 수면 1,913개, 국경 2,964개, 장소 2,256개 (타일별 중복 포함) |
| 화면 검증 | 세계 z1.3, 유럽 z6.4, 한국 z6.4 Metal 캡처 모두 타일 오류 0개 |
| 데이터 선택 | 유럽·동아시아 z6.4에서 상세 최대 z7 선택 확인. 동아시아 타일은 기존 1:10m, 유럽 타일은 1:50m 파일 선택을 자동 테스트로 확인 |
| 직접 측량 | 현장 기록 0건. [직접 기록 전용 모드](FIRST_PARTY_MAP.md)와 세계지도 합성은 아직 구현하지 않음 |

비교 화면: [세계](../artifacts/world-map/world.png), [유럽](../artifacts/world-map/europe.png), [동아시아](../artifacts/world-map/east-asia.png).

## 재생성·출처

1:50m의 네 GeoJSON은 기존 1:110m·1:10m 원본과 같은 [`nvkelso/natural-earth-vector` 고정 커밋 `ca96624a56bd078437bca8184e78163e5039ad19`](https://github.com/nvkelso/natural-earth-vector/tree/ca96624a56bd078437bca8184e78163e5039ad19/geojson)에서 받았다.

| 입력 파일 (`assets/map/source/50m/`) | SHA-256 |
|---|---|
| `ne_50m_land.geojson` | `e874b27a51d146452be360cafb3cc50c86001074a67d534113e6534682f9826b` |
| `ne_50m_lakes.geojson` | `d350b75978b26fe839b797c2c529b2fb8f47fb3983c03f4964e36d5df9378a52` |
| `ne_50m_admin_0_boundary_lines_land.geojson` | `2faac4f6b34386f3d21b6e018cf151f241f00e5c936d44dd17d7d9bfb147fa48` |
| `ne_50m_populated_places.geojson` | `da4662b7bbfeb897d02f228c5839131dce27acff5717630f91ccff4f67828ee7` |

출력 `assets/map/world_50m.pmtiles` SHA-256: `e51f203900abfaa60a63e66f3de74c41c1759757d44c0419c3df4af60f5a9892`. 기존 1:110m·동아시아 1:10m 파일의 원본·해시는 [MAP_DATA.md](MAP_DATA.md)에 있다. 재빌드는 데모가 파일을 열고 있지 않을 때 실행한다.

```bash
cargo run -p mappa-map-data --bin build_world_50m
cargo run -p mappa-map-data --bin audit_fixture -- assets/map/world_50m.pmtiles
cargo run -p mappa-map-demo
```

다음 범위는 단순 타일 확장이 아니라 실제 데이터 요건으로 판단한다. 전 세계 도로·역·기관을 보이려면 해당 범위의 실제 위치 원본과 이용 조건, 연결성·최신성 검증이 필요하다. 직접 측량만으로 채울 구역은 관측 범위를 명시하고 세계 개략 지형 위에 별도 출처 레이어로 합성하는 작업이 남아 있다.
