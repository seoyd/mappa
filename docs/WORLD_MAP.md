# Mappa 오프라인 세계지도 — 2026-09-24

## 현재 판정

**세계 개략 지도는 표시된다. 전 세계 상세 지도는 완성되지 않았다.** 기본 화면은 Mappa의 Rust 타일 빌더·PMTiles 리더·Metal 렌더러가 로컬 파일만 읽는다. 해안·국경·도시·일부 주요 도로의 좌표 원본은 [Natural Earth 퍼블릭 도메인 자료](https://www.naturalearthdata.com/about/terms-of-use/)다. Mappa가 전 세계를 직접 측량했거나 독자적 원본 지형을 확보했다는 뜻은 아니다. 지도 서버와 유료 지도 API는 런타임에서 사용하지 않는다.

## 실제로 보이는 범위

| 화면 타일 단계 | 적용 지역 | 원본과 표시 내용 | 한계 |
|---|---|---|---|
| z0–z4 | Web Mercator 세계 범위 | Natural Earth 1:110m 대륙·호수·국경·나라 이름 | 축소 지도 |
| z5–z7 | Web Mercator 세계 범위 | Natural Earth 1:10m 해안·호수·국경, z6부터 선별된 도시·주요 도로 | 1:10m은 **축척 1:1,000만**이며 10m 위치 정확도가 아님 |
| z5–z7 | 동아시아 일부 | 기존 1:10m 지역 아카이브 우선 선택 | 같은 종류의 개략 원본이며 별도 정밀 측량이 아님 |
| z8 이상 | 전 세계 | z7 데이터를 확대 | 새 상세 객체가 추가되지 않음 |

Web Mercator 표현 범위는 극점 밖 위도 약 ±85.05°까지다. z5–z7의 도로는 원본에 들어 있는 선별된 선만 그린다. [Natural Earth의 도로 설명](https://www.naturalearthdata.com/downloads/10m-cultural-vectors/roads/)도 기본 도로의 북미 중심 범위와 전 세계 확장 필요성을 명시한다. 파리·라고스·부에노스아이레스·시드니 캡처에는 실제 원본에 수록된 도로선이 보였으나, 이 사례로 모든 나라의 도로가 완전하다고 판단하지 않는다. 건물·역·공공기관·주소·길찾기·현재 도로 연결성은 전 세계 지도에 없다.

## 이번 확장과 검증

기존 기본 세계 상세 파일은 1:50m(`world_50m.pmtiles`)이었다. 고정된 같은 Natural Earth 커밋의 1:10m 원본 5개를 받아 해시를 확인하고 전 세계 z5–z7 아카이브 `world_10m.pmtiles`를 새로 생성했다. Rust 빌더에는 공간 인덱스를 넣어 각 타일과 겹치는 형상만 검사한다. 지형 좌표를 그럴듯하게 하드코딩하거나 보간해서 새 도로를 만들지 않았다.

| 검증 항목 | 관측 결과 |
|---|---|
| 빌드 원천 | 해안/육지·호수·육상 국경·도시·도로 5개; [`world_10m_sources.toml`](../data/world_10m_sources.toml)에 URL·SHA-256·권리·범위 고정 |
| 산출물 | 11,021개 비어 있지 않은 타일, 타일별 중복 포함 피처 116,946개, 6,121,010바이트, SHA-256 `edc42302346aa360cc16b2eb5cf54c11fa7db8c7d927b2911e44e147f7ee78ca` |
| 전체 타일 감사 | 11,021개 모두 디코드, 오류 0; 비어 있는 해양 타일 10,483개. 타일별 중복 포함 육지 34,802, 호수 5,649, 국경 25,762, 주요 도로 46,385, 장소 4,890 |
| 화면 | 동일 파리 z6.4에서 기존 50m [이전 화면](../artifacts/world-10m/europe-50m-before.png)과 [10m 화면](../artifacts/world-10m/europe-10m.png) 캡처, 두 화면 모두 타일 오류 0. 10m은 해안선·주요 도로·도시가 더 보임 |
| 대륙별 표시 | [서아프리카](../artifacts/world-10m/africa-10m.png), [남미](../artifacts/world-10m/south-america-10m.png), [호주](../artifacts/world-10m/australia-10m.png) Mac Metal 캡처에서 타일 오류 0 |
| 날짜변경선 | [피지 z7.4](../artifacts/world-10m/fiji-dateline-10m.png) 캡처가 실제 z7 세계 타일을 선택, 오류 0. 경도 래핑 때문에 z4로 후퇴하던 선택 조건을 수정 |

이 감사는 파일 해독과 표시 확인이다. 실제 현장 정확도, 도로 완전성, 국가별 건물 커버리지의 독립 검증은 아니다. 원본 도로가 없는 지역을 새 선으로 채우지 않는다. 기존 50m 파일은 비교·회귀용으로 보관하고 기본 상세 선택만 10m으로 바꿨다.

Mac Metal 1200×720, 파리 중심 z6.4의 같은 비동기 팬 40프레임을 파일별로 3회 실행했다. 실행별 중앙값의 중앙값은 기존 50m **2.442ms**, 새 10m **3.508ms**였다. 초기 로딩의 3회 중앙값은 각각 **36.472ms**, **45.271ms**였다. 각 실행에서 미해결 화면 타일 0, 실패 0이었다. 더 많은 형상을 그리는 비용이 관측되며, 이 단일 Mac·카메라 측정으로 iPhone 프레임 성능을 추정하지 않는다.

## 재생성

입력은 [`nvkelso/natural-earth-vector` 고정 커밋 `ca96624a56bd078437bca8184e78163e5039ad19`](https://github.com/nvkelso/natural-earth-vector/tree/ca96624a56bd078437bca8184e78163e5039ad19/geojson)의 GeoJSON이다. `data/world_10m_sources.toml`의 URL로 파일 5개를 한 디렉터리에 받아 둔다. 소스 확인과 타일 빌드는 Rust CLI가 수행한다. 검증된 입력 이외의 파일은 해시 오류로 거절된다.

```bash
cargo run --release --offline -p mappa-map-data --bin build_world_10m -- \
  data/world_10m_sources.toml /path/to/10m-geojson assets/map/world_10m.pmtiles
cargo run --release --offline -p mappa-map-data --bin audit_fixture -- assets/map/world_10m.pmtiles
```

이 원본 파일들은 제품 런타임에 필요하지 않으며 Git 저장소에는 빌드 산출물과 출처 기록만 둔다. 1:10m 지형은 Mappa 전 세계 canonical GeoDB로 아직 편입되지 않았다. [나주 지역 canonical 실증의 S16 결과](MAP_V0_3C_GATE_AUDIT.md)도 여전히 `NO_GO_SOURCE_COVERAGE`다. 이후 국가별 승인 원천·건물·도로 연결성·독립 기준점을 확보하고, 구축 완료 구역과 빈 구역을 별도로 표시해야 한다.
