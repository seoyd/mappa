# Mappa 오프라인 세계지도 — 2026-09-24

## 현재 판정

**세계 개략 지도는 표시된다. 전 세계 상세 지도는 완성되지 않았다.** 기본 화면은 Mappa의 Rust 타일 빌더·PMTiles 리더·Metal 렌더러가 로컬 파일만 읽는다. 해안·국경·도시·일부 주요 도로의 좌표 원본은 [Natural Earth 퍼블릭 도메인 자료](https://www.naturalearthdata.com/about/terms-of-use/)다. Mappa가 전 세계를 직접 측량했거나 독자적 원본 지형을 확보했다는 뜻은 아니다. 지도 서버와 유료 지도 API는 런타임에서 사용하지 않는다.

상세 원천 확대의 첫 실증은 [모나코 건물 구축 현황](WORLD_BUILDINGS_PROGRESS.md)에 기록했다. 승인된 모나코 건물과 맨해튼 도로 파일을 기본 세계 모드의 고배율에 연결했다. 두 작은 실증 구역 밖의 상세 지리는 구축되지 않았다.
사용자가 선택한 기존 공유조건 금지 규칙에 따라 도로는 [국가별 공식 원천 실증](WORLD_ROADS_PROGRESS.md)으로 진행한다. 미국 뉴욕 카운티 한 파일을 처리했으며 세계 상세 도로망은 아직 없다.

## 실제로 보이는 범위

| 화면 타일 단계 | 적용 지역 | 원본과 표시 내용 | 한계 |
|---|---|---|---|
| z0–z4 | Web Mercator 세계 범위 | Natural Earth 1:110m 대륙·호수·국경·나라 이름 | 축소 지도 |
| z5–z7 | Web Mercator 세계 범위 | Natural Earth 1:10m 해안·호수·국경, z6부터 선별된 도시·주요 도로·큰 수계, z7에서 더 많은 수계 | 1:10m은 **축척 1:1,000만**이며 10m 위치 정확도가 아님 |
| z8–z9 | 전 세계 | z7 개략 데이터를 확대 | 새 상세 객체가 추가되지 않음 |
| z10–z13 | 모나코 | z7 개략 데이터를 확대 | 건물 파일에 실제 타일이 없는 배율; 위치·객체 상세는 증가하지 않음 |
| z10–z15 | 모나코·맨해튼 실증 구역 | 승인된 로컬 건물·도로 PMTiles; 건물은 z14부터 | 둘 다 부분 자료이며 위치 정확도·완전성 미통과 |
| z10 이상 | 미수집 지역 | 중립색 빈 화면 | 지리 정보를 추정해 채우지 않음 |

Web Mercator 표현 범위는 극점 밖 위도 약 ±85.05°까지다. z5–z7의 도로는 원본에 들어 있는 선별된 선만 그린다. [Natural Earth의 도로 설명](https://www.naturalearthdata.com/downloads/10m-cultural-vectors/roads/)도 기본 도로의 북미 중심 범위와 전 세계 확장 필요성을 명시한다. [수계 원본](https://www.naturalearthdata.com/downloads/10m-physical-vectors/10m-rivers-lake-centerlines/)도 개략화된 중심선이며 모든 하천을 포함하지 않는다. 파리·라고스·부에노스아이레스·시드니 캡처에는 실제 원본에 수록된 도로선이 보였으나, 이 사례로 모든 나라의 도로가 완전하다고 판단하지 않는다. 건물과 정밀 도로는 각각 한 실증 구역에만 있으며, 역·공공기관·주소·길찾기·검증된 도로 연결성은 전 세계 범위에 없다.

## 이번 확장과 검증

### 승인된 지역 패키지 결합 — 2026-09-24

기본 세계 모드가 [지역 목록](../assets/map/regional_packs.toml)의 각 아카이브를 함께 연다. 시작할 때 canonical 출처 manifest의 라이선스 게이트를 통과시키고, PMTiles의 범위·출처 표기를 manifest와 대조한다. 고배율 타일과 겹치는 여러 지역 파일을 읽어 타입별 레이어를 합친다. 중복 지형의 식별·병합은 아직 구현되지 않았으므로 겹치는 지역 자료를 추가하기 전에 별도 검증이 필요하다. 현재 두 파일은 서로 다른 지역이다.

Mac Metal 기본 세계 모드 캡처에서 [모나코 건물](../artifacts/world-integration/monaco-world.png)과 [맨해튼 도로](../artifacts/world-integration/manhattan-world.png)가 실제 z14 타일로 표시됐고, [파리의 미수집 상세 영역](../artifacts/world-integration/unmapped-world.png)은 빈 중립색으로 표시됐다. [모나코 z12](../artifacts/world-integration/monaco-overview-z12.png)는 실제 건물 타일이 시작되기 전이라 기존 z7 개략 자료를 확대하며 화면에 그 한계를 밝힌다. 네 캡처의 타일 실패는 각각 0개다. 최종 모나코의 동기·비동기 캡처 PNG는 SHA-256 `6477b6f1bb2cf1b12b861cdfce308e0de0a0b05e4db2aa7021989c6b73e074e2`로 일치했다. 이는 자료 선택·표시의 검증이며 현장 위치 정확도나 전 세계 커버리지 통과 판정이 아니다.

현재 목록은 파일 2개를 시작 시 모두 열고 타일 요청마다 목록을 검사한다. 전 세계 수만 개 분할 파일에 적용하려면 공간 인덱스와 지연 열기, 파일 핸들·캐시 한도를 추가해야 한다. iPhone 구동과 메모리 측정도 아직 없다.

### Natural Earth 전 세계 개략 자료

기존 기본 세계 상세 파일은 1:50m(`world_50m.pmtiles`)이었다. 고정된 같은 Natural Earth 커밋의 1:10m 원본 6개를 받아 해시를 확인하고 전 세계 z5–z7 아카이브 `world_10m.pmtiles`를 새로 생성했다. Rust 빌더는 공간 인덱스로 각 타일과 겹치는 형상만 검사한다. 수계는 원본 `scalerank`로 z6에 0–5, z7에 0–7 등급을 선택했다. 지형 좌표를 하드코딩하거나 보간해서 새 도로·강을 만들지 않았다.

| 검증 항목 | 관측 결과 |
|---|---|
| 빌드 원천 | 해안/육지·호수·육상 국경·도시·도로·수계 중심선 6개; [`world_10m_sources.toml`](../data/world_10m_sources.toml)에 URL·SHA-256·권리·범위 고정 |
| 산출물 | 11,021개 비어 있지 않은 타일, 타일별 중복 포함 피처 121,611개, 6,742,358바이트, SHA-256 `fd0b6c235c8102f3dfd0477e08743ec1a3d85794253d91075070a13efe3fda32` |
| 전체 타일 감사 | 11,021개 모두 디코드, 오류 0; 비어 있는 해양 타일 10,483개. 타일별 중복 포함 육지 34,802, 호수 5,649, 국경 25,762, 하천 선분 5,304, 주요 도로 46,385, 장소 4,890 |
| 화면 | 동일 파리 z6.4에서 기존 50m [이전 화면](../artifacts/world-10m/europe-50m-before.png)과 [10m 화면](../artifacts/world-10m/europe-10m.png) 캡처, 두 화면 모두 타일 오류 0. 10m은 해안선·주요 도로·도시가 더 보임 |
| 대륙별 표시 | [서아프리카](../artifacts/world-10m/africa-10m.png), [남미](../artifacts/world-10m/south-america-10m.png), [호주](../artifacts/world-10m/australia-10m.png) Mac Metal 캡처에서 타일 오류 0 |
| 날짜변경선 | [피지 z7.4](../artifacts/world-10m/fiji-dateline-10m.png) 캡처가 실제 z7 세계 타일을 선택, 오류 0. 경도 래핑 때문에 z4로 후퇴하던 선택 조건을 수정 |
| 수계 화면 | [아마존 z6](../artifacts/world-10m/amazon-rivers-z6.png), [나일강 z7](../artifacts/world-10m/nile-rivers-z7.png), [동아시아 세계 파일 z7](../artifacts/world-10m/east-asia-world-file-z7.png) Mac Metal 캡처, 타일 오류 0 |
| 레이어 존재 현황 | [z7 타일의 z3 구역별 CSV](../artifacts/world-10m/tile-presence-z7-by-z3.csv): 64구역 중 육지 타일 61, 수계선 타일 29, 도로선 타일 25, 육지는 있지만 도로선 타일은 없는 구역 36. 이는 자료 **존재 여부**이며 국가별 완성률이 아님 |

이 감사는 파일 해독과 표시 확인이다. 실제 현장 정확도, 도로·수계 완전성, 국가별 건물 커버리지의 독립 검증은 아니다. 원본 선이 없는 지역을 새 선으로 채우지 않는다. 기존 50m와 동아시아 10m 파일은 비교·회귀용으로 보관한다. 기본 세계 모드는 세계 110m·10m 파일과 [지역 목록](../assets/map/regional_packs.toml)의 승인된 PMTiles 두 개를 연다. 목록은 출처 manifest의 비공유조건 권리 판정·지리 범위·출처 표기가 아카이브와 일치하는지 시작할 때 확인한다.

동일한 Mac Metal 1200×720, 파리 중심 z6.4의 비동기 팬 40프레임을 기존 5개 원본 세계 파일과 이번 6개 원본 세계 파일에서 각각 3회 실행했다. 각 실행의 프레임 중앙값은 기존 **3.433/3.615/3.545ms**, 이번 **3.871/3.939/3.740ms**로, 실행별 중앙값의 중앙값은 **3.545 → 3.871ms**였다. 초기 로딩은 기존 **43.388/45.908/45.944ms**, 이번 **48.093/46.988/44.677ms**로 중앙값 **45.908 → 46.988ms**였다. 양쪽 모두 미해결 화면 타일과 실패 0. 수계선 추가의 비용이 이 장면에서 관측됐다. 단일 Mac·장면 측정이며 iPhone GPU/메모리 성능은 측정되지 않았다.

## 재생성

입력은 [`nvkelso/natural-earth-vector` 고정 커밋 `ca96624a56bd078437bca8184e78163e5039ad19`](https://github.com/nvkelso/natural-earth-vector/tree/ca96624a56bd078437bca8184e78163e5039ad19/geojson)의 GeoJSON이다. `data/world_10m_sources.toml`의 URL로 파일 6개를 한 디렉터리에 받아 둔다. 소스 확인과 타일 빌드는 Rust CLI가 수행한다. 검증된 입력 이외의 파일은 해시 오류로 거절된다.

```bash
cargo run --release --offline -p mappa-map-data --bin build_world_10m -- \
  data/world_10m_sources.toml /path/to/10m-geojson assets/map/world_10m.pmtiles
cargo run --release --offline -p mappa-map-data --bin audit_fixture -- assets/map/world_10m.pmtiles
```

이 원본 파일들은 제품 런타임에 필요하지 않으며 Git 저장소에는 빌드 산출물과 출처 기록만 둔다. 1:10m 지형은 Mappa 전 세계 canonical GeoDB로 아직 편입되지 않았다. [나주 지역 canonical 실증의 S16 결과](MAP_V0_3C_GATE_AUDIT.md)도 여전히 `NO_GO_SOURCE_COVERAGE`다. 이후 국가별 승인 원천·건물·도로 연결성·독립 기준점을 확보하고, 구축 완료 구역과 빈 구역을 별도로 표시해야 한다.

같은 manifest와 원본으로 두 번 생성한 PMTiles의 SHA-256은 모두 `fd0b6c235c8102f3dfd0477e08743ec1a3d85794253d91075070a13efe3fda32`였다.
