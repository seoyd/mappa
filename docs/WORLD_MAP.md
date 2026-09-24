# Mappa 오프라인 세계지도 — 2026-09-25

## 현재 판정

**세계 개략 지도는 표시된다. 전 세계 상세 지도는 완성되지 않았다.** 기본 화면은 Mappa의 Rust 타일 빌더·PMTiles 리더·Metal 렌더러가 로컬 파일만 읽는다. 해안·국경·도시·일부 주요 도로의 좌표 원본은 [Natural Earth 퍼블릭 도메인 자료](https://www.naturalearthdata.com/about/terms-of-use/)다. Mappa가 전 세계를 직접 측량했거나 독자적 원본 지형을 확보했다는 뜻은 아니다. 지도 서버와 유료 지도 API는 런타임에서 사용하지 않는다.

상세 원천 확대는 [모나코·Queens 건물](WORLD_BUILDINGS_PROGRESS.md), [뉴욕주 62개 카운티 도로](NY_STATE_ROADS_PROGRESS.md), [뉴저지주 21개 카운티 도로](NJ_STATE_ROADS_PROGRESS.md), [델라웨어주 3개 카운티 도로](DE_STATE_ROADS_PROGRESS.md), [코네티컷주 9개 계획지역 도로](CT_STATE_ROADS_PROGRESS.md), [로드아일랜드주 5개 카운티 도로](RI_STATE_ROADS_PROGRESS.md), [매사추세츠주 14개 카운티 도로](MA_STATE_ROADS_PROGRESS.md)·[수면](MA_STATE_WATER_PROGRESS.md), [뉴햄프셔주 10개 카운티 도로·수면](NH_STATE_PROGRESS.md), [버몬트주 14개 카운티 도로·수면](VT_STATE_PROGRESS.md), [메인주 16개 카운티 도로·수면](ME_STATE_PROGRESS.md), [메릴랜드주 24개 카운티 상당 단위 도로·수면](MD_STATE_PROGRESS.md), [펜실베이니아주 67개 카운티 도로·수면](PA_STATE_PROGRESS.md), [웨스트버지니아주 55개 카운티 도로·수면](WV_STATE_PROGRESS.md), [오하이오주 88개 카운티 도로·수면](OH_STATE_PROGRESS.md), [워싱턴 DC 도로·수면](DC_PROGRESS.md), [프랑스 IGN 파리 도로](FR_PARIS_ROADS_PROGRESS.md)·[수면](FR_PARIS_WATER_PROGRESS.md)·[여객역](FR_PARIS_STATIONS_PROGRESS.md), [GB 공식 RoadLink 52개 격자](GB_ROADS_PROGRESS.md), [캐나다 13개 지역 NRN 도로](CA_ROADS_PROGRESS.md), [뉴욕시 5개 카운티 수면](WORLD_WATER_PROGRESS.md), [뉴욕시 주변 공원 경계](WORLD_PARKS_PROGRESS.md)에 실증했다. 기본 세계 모드는 기존 34개, GB 52개, 캐나다 15개, 총 101개 지역 패키지를 필요할 때 결합한다. 이 지역 밖의 상세 지리는 구축되지 않았다.
사용자가 선택한 기존 공유조건 금지 규칙에 따라 도로는 국가별 공식 원천으로 진행한다. 미국 뉴욕주·뉴저지주·델라웨어주·코네티컷주·로드아일랜드주·매사추세츠주·뉴햄프셔주·버몬트주·메인주·메릴랜드주·펜실베이니아주·웨스트버지니아주·오하이오주·워싱턴 DC 389개 카운티 상당 단위, 프랑스 IGN 파리 원천 범위, Great Britain의 OS Open Roads RoadLink, 캐나다 13개 지역의 NRN Road Segment 원천을 처리했으며 세계 상세 도로망은 아직 없다.

## 실제로 보이는 범위

| 화면 타일 단계 | 적용 지역 | 원본과 표시 내용 | 한계 |
|---|---|---|---|
| z0–z4 | Web Mercator 세계 범위 | Natural Earth 1:110m 대륙·호수·국경·나라 이름 | 축소 지도 |
| z5–z7 | Web Mercator 세계 범위 | Natural Earth 1:10m 해안·호수·국경, z6부터 선별된 도시·주요 도로·큰 수계, z7에서 더 많은 수계 | 1:10m은 **축척 1:1,000만**이며 10m 위치 정확도가 아님 |
| z8–z9 | 전 세계 | z7 개략 데이터를 확대 | 새 상세 객체가 추가되지 않음 |
| z10–z13 | 모나코 | z7 개략 데이터를 확대 | 건물 파일에 실제 타일이 없는 배율; 위치·객체 상세는 증가하지 않음 |
| z10–z15 | 뉴욕주 62개 카운티 원천 범위 | Census 2025 도로 지역 PMTiles | 도로 연결성·독립 위치 정확도·완전성 미통과 |
| z10–z15 | 뉴저지주 21개 카운티 원천 범위 | Census 2025 도로 지역 PMTiles | 주 경계 중복·연결성·독립 위치 정확도 미검증 |
| z10–z15 | 델라웨어주 3개 카운티 원천 범위 | Census 2025 도로 지역 PMTiles | 도로 완전성·연결성·독립 위치 정확도 미검증 |
| z10–z15 | 코네티컷주 9개 계획지역 원천 범위 | Census 2025 도로 지역 PMTiles | 주 경계 연결성·독립 위치 정확도 미검증 |
| z10–z15 | 로드아일랜드주 5개 카운티 원천 범위 | Census 2025 도로 지역 PMTiles | 주 경계 연결성·독립 위치 정확도 미검증 |
| z10–z15 | 매사추세츠주 14개 카운티 원천 범위 | Census 2025 도로·수면 지역 PMTiles | 주 경계 연결성·독립 위치 정확도 미검증 |
| z10–z15 | 뉴햄프셔주 10개 카운티 원천 범위 | Census 2025 도로·수면 지역 PMTiles | 주 경계 연결성·독립 위치 정확도 미검증 |
| z10–z15 | 버몬트주 14개 카운티 원천 범위 | Census 2025 도로·수면 지역 PMTiles | 주 경계 연결성·독립 위치 정확도 미검증 |
| z10–z15 | 메인주 16개 카운티 원천 범위 | Census 2025 도로·수면 지역 PMTiles | 주 경계 연결성·독립 위치 정확도 미검증 |
| z10–z15 | 메릴랜드주 24개 카운티 상당 단위 원천 범위 | Census 2025 도로·수면 지역 PMTiles | 주 경계 연결성·독립 위치 정확도 미검증 |
| z10–z15 | 펜실베이니아주 67개 카운티 원천 범위 | Census 2025 도로 2개·수면 1개 지역 PMTiles | 주 경계 연결성·독립 위치 정확도 미검증 |
| z10–z15 | 웨스트버지니아주 55개 카운티 원천 범위 | Census 2025 도로 2개·수면 1개 지역 PMTiles | 주 경계 연결성·독립 위치 정확도 미검증 |
| z10–z15 | 오하이오주 88개 카운티 원천 범위 | Census 2025 도로 3개·수면 1개 지역 PMTiles | 주 경계 연결성·독립 위치 정확도 미검증 |
| z10–z15 | 워싱턴 DC 1개 카운티 상당 단위 원천 범위 | Census 2025 도로·수면 지역 PMTiles | 메릴랜드 경계 연결성·독립 위치 정확도 미검증 |
| z10–z15 | 프랑스 IGN BD TOPO 파리 D075 원천 범위 | 2026-06-15 도로 중심선·내륙 수면과 z13부터 `FICTIF=Non` 여객역 지역 PMTiles | 연결성·RGF93/WGS84 독립 위치 정확도 미검증; 건물·공공시설 미구축 |
| z10–z15 | Great Britain의 OS RoadLink 52개 격자 | OS Open Roads 2026-04 일반화 도로 지역 PMTiles | 북아일랜드 제외; 연결성·현장 위치 정확도·모든 실제 도로의 완전성 미검증 |
| z10–z15 | 캐나다 13개 주·준주 | 공식 NRN Road Segment 지역 PMTiles 15개 | 주 경계 연결성·현장 위치 정확도 미검증 |
| z10–z15 | 뉴욕시 5개 카운티 | Census 2025 수면 지역 PMTiles | 뉴욕주 전체 수면은 없음 |
| z12–z15 | 뉴욕시 원천 범위 사각형 | Census 2025 공원·휴양 구역 경계 | 실제 수목 피복·공원 목록의 완전성은 아님 |
| z14–z15 | 모나코 및 Queens 동부 한 z11 타일 범위 | Microsoft 건물 지역 PMTiles; Queens에서는 도로·수면과 합성 | 건물 자료의 시기·위치 정확도·완전성 미검증 |
| z10 이상 | 미수집 지역 | 중립색 빈 화면 | 지리 정보를 추정해 채우지 않음 |

Web Mercator 표현 범위는 극점 밖 위도 약 ±85.05°까지다. z5–z7의 도로는 원본에 들어 있는 선별된 선만 그린다. [Natural Earth의 도로 설명](https://www.naturalearthdata.com/downloads/10m-cultural-vectors/roads/)도 기본 도로의 북미 중심 범위와 전 세계 확장 필요성을 명시한다. [수계 원본](https://www.naturalearthdata.com/downloads/10m-physical-vectors/10m-rivers-lake-centerlines/)도 개략화된 중심선이며 모든 하천을 포함하지 않는다. 파리·라고스·부에노스아이레스·시드니 캡처에는 실제 원본에 수록된 도로선이 보였으나, 이 사례로 모든 나라의 도로가 완전하다고 판단하지 않는다. 건물·정밀 도로·정밀 수면은 위 지역 실증 범위에만 있고, 역·공공기관·주소·길찾기·검증된 도로 연결성은 전 세계 범위에 없다.

## 이번 확장과 검증

### 승인된 지역 패키지 결합 — 2026-09-24

기본 세계 모드가 [기존 지역 목록](../assets/map/regional_packs.toml), [GB 도로 목록](../assets/map/gb_regional_packs.toml), [캐나다 도로 목록](../assets/map/ca_regional_packs.toml)의 아카이브를 선택할 수 있다. 시작할 때 canonical 출처 manifest의 라이선스 게이트를 통과시키고, PMTiles는 해당 지역의 첫 타일 요청 시 열어 범위·출처 표기를 manifest와 대조한다. 고배율 타일과 겹치는 여러 지역 파일을 읽어 타입별 레이어를 합친다. 뉴욕주 도로와 뉴욕시 수면·공원·Queens 건물은 실제 같은 타일에서 합쳐진다. GB 원천 격자의 동일 도로 ID는 빌드 중 한 격자에 귀속한다. 다른 원천 간 같은 종류 자료의 중복 식별·병합은 아직 구현되지 않았다.

### 뉴욕주 도로 확장 — 2026-09-24

[뉴욕주 62개 카운티 원천 감사](NY_STATE_ROADS_PROGRESS.md)에서 도로 340,381개를 채택하고 39,266개 레코드를 거절했다. 171,167개 비어 있지 않은 타일을 모두 해독했으며 오류는 0개였다. 기존 뉴욕시 도로 팩의 비어 있지 않은 1,522개 타일은 새 팩에도 있고, 기존 도로 선형이 빠진 타일은 0개였다. 기본 세계 모드 [Buffalo z14.4](../artifacts/world-roads/ny-state/buffalo-world-z14.png)와 [Queens 네 레이어 z14.4](../artifacts/world-roads/ny-state/queens-four-layers-world.png) 캡처에서 타일 실패가 각각 0개였다. 이는 파일·표시 검증이며 도로 연결성이나 현장 위치 정확도 검증은 아니다. 뉴욕시 5개 카운티 도로 팩은 [이전 실증](WORLD_ROADS_PROGRESS.md) 비교용으로 보관한다.

### 뉴저지주 도로 확장 — 2026-09-24

[뉴저지주 21개 카운티 원천 감사](NJ_STATE_ROADS_PROGRESS.md)에서 도로 177,815개를 채택하고 6,537개 레코드를 거절했다. 29,097개 비어 있지 않은 타일을 모두 해독했으며 오류는 0개였다. 기본 세계 모드 [Newark z14.4](../artifacts/world-roads/nj-state/newark-world-z14.png) 캡처의 타일 실패도 0개였다. 두 주 팩의 같은 타일 214개에서 완전 동일한 도로 좌표열은 0개였지만 부분 중복과 경계 연결성은 미검증이다.

### 델라웨어주 도로 확장 — 2026-09-24

[Census 공식 ZIP 3개 감사](DE_STATE_ROADS_PROGRESS.md)에서 도로 32,518개를 채택하고 877개 레코드를 거절했다. 지역 팩의 실제 타일 7,409개를 모두 해독해 오류 0개였고, 기본 세계 모드 [Dover z14](../artifacts/world-roads/de-state/dover-world-z14.png) 캡처의 타일 실패도 0개였다. 주 경계 중복·연결성과 독립 위치 정확도는 미검증이다.

### 코네티컷주 도로 확장 — 2026-09-24

[Census 2025 계획지역 ZIP 9개 감사](CT_STATE_ROADS_PROGRESS.md)에서 도로 87,833개를 채택하고 7,010개 레코드를 거절했다. 지역 팩의 실제 타일 20,398개를 전부 해독해 오류 0개였고, 기본 세계 모드 [Hartford z14](../artifacts/world-roads/ct-state/hartford-world-z14.png) 캡처의 타일 실패도 0개였다. 2025 원천의 9개 계획지역을 그대로 사용했으며 옛 8개 카운티 구획으로 추정하지 않았다.

### 로드아일랜드주 도로 확장 — 2026-09-24

[Census 공식 ZIP 5개 감사](RI_STATE_ROADS_PROGRESS.md)에서 도로 29,931개를 채택하고 752개 레코드를 거절했다. 지역 팩의 실제 타일 4,867개를 전부 해독해 오류 0개였고, 기본 세계 모드 [Providence z14](../artifacts/world-roads/ri-state/providence-world-z14.png) 캡처의 타일 실패도 0개였다. 주 경계 연결성과 독립 위치 정확도는 미검증이다.

### 매사추세츠주 도로 확장 — 2026-09-25

[Census 공식 ZIP 14개 감사](MA_STATE_ROADS_PROGRESS.md)에서 도로 210,165개를 채택하고 6,610개 레코드를 거절했다. 지역 팩의 실제 타일 33,938개를 모두 해독해 오류 0개였고, 기본 세계 모드 [Boston z14](../artifacts/world-roads/ma-state/boston-world-z14.png) 캡처의 타일 실패도 0개였다. 인접 주 경계 연결성과 현장 위치 정확도는 미검증이다.

### 매사추세츠주 수면 확장 — 2026-09-25

[Census 공식 수면 ZIP 14개 감사](MA_STATE_WATER_PROGRESS.md)에서 폴리곤 23,633개를 채택하고 모호한 내부 링 18개를 거절했다. 별도 지역 팩의 실제 타일 31,357개를 전수 해독해 실패 0개였고, 기본 세계 모드 [Boston 도로·수면 z14](../artifacts/world-water/ma-state/boston-road-water-z14.png) 화면의 타일 실패도 0개였다. 해안·수면의 독립 위치 정확도와 건물·시설은 미검증이다.

### 뉴햄프셔주 도로·수면 확장 — 2026-09-25

[Census 공식 ZIP 20개 감사](NH_STATE_PROGRESS.md)에서 도로 68,617개와 수면 폴리곤 5,897개를 채택하고, 각각 12,809개·7개를 제외했다. 도로 32,213개와 수면 15,406개 실제 타일을 전수 해독해 모두 실패 0개였다. 기본 세계 모드 [Concord 도로·수면 z14](../artifacts/world-water/nh-state/concord-road-water-z14.png)에서도 타일 실패는 0개였다. 현장 위치 정확도와 인접 주 경계 연결성은 미검증이다.

### 버몬트주 도로·수면 확장 — 2026-09-25

[Census 공식 ZIP 28개 감사](VT_STATE_PROGRESS.md)에서 도로 43,060개와 수면 폴리곤 14,567개를 채택하고, 각각 9,712개·2개를 제외했다. 도로 33,631개와 수면 21,740개 실제 타일을 전수 해독해 모두 실패 0개였다. 기본 세계 모드 [Burlington 도로·수면 z14](../artifacts/world-water/vt-state/burlington-road-water-z14.png)에서도 타일 실패는 0개였다. 주 경계 연결성과 독립 위치 정확도는 미검증이다.

### 메인주 도로·수면 확장 — 2026-09-25

[Census 공식 ZIP 32개 감사](ME_STATE_PROGRESS.md)에서 도로 118,918개와 수면 8,665개를 채택하고 각각 16,270개·14개를 제외했다. Rust가 공식 배포 목록을 직접 읽어 16개 카운티 파일 이름과 목록 해시를 고정했다. 도로 102,440개와 수면 57,555개 실제 타일을 전수 해독해 실패 0개였다. 기본 세계 모드 [Portland 도로·수면 z14](../artifacts/world-water/me-state/portland-road-water-z14.png)의 타일 실패도 0개였다. 현장 위치 정확도와 카운티·주 경계 연결성은 미검증이다.

### 메릴랜드주 도로·수면 확장 — 2026-09-25

[Census 공식 ZIP 48개 감사](MD_STATE_PROGRESS.md)에서 도로 212,347개와 수면 14,888개를 채택하고 각각 11,655개·17개를 제외했다. Rust가 공식 디렉터리에서 도로·수면의 일치하는 24개 카운티 상당 단위 목록을 고정했다. 도로 38,627개와 수면 29,258개 실제 타일을 전수 해독해 실패 0개였다. 기본 세계 모드 [Baltimore 도로·수면 z14](../artifacts/world-water/md-state/baltimore-road-water-z14.png)의 타일 실패도 0개였다. 독립 위치 정확도와 경계 연결성은 미검증이다.

### 펜실베이니아주 도로·수면 확장 — 2026-09-25

[Census 공식 ZIP 134개 감사](PA_STATE_PROGRESS.md)에서 도로 490,912개와 수면 46,883개를 채택하고 각각 64,747개·26개를 제외했다. Rust가 공식 디렉터리의 도로·수면 67개 카운티 목록 일치를 확인했다. 도로 원본의 실제 타일 167,144개를 압축 바이트 동일한 서부 86,243개·동부 80,901개로 나누고 두 팩과 수면 63,699개 타일을 전수 해독해 실패 0개였다. 기본 세계 모드 [Pittsburgh](../artifacts/world-roads/pa-state/pittsburgh-road-water-z14.png)와 [Philadelphia](../artifacts/world-roads/pa-state/philadelphia-road-water-z14.png) z14 도로·수면 화면도 각각 타일 실패 0개였다. 독립 위치 정확도와 경계 연결성은 미검증이다.

### 웨스트버지니아주 도로·수면 확장 — 2026-09-25

[Census 공식 ZIP 110개 감사](WV_STATE_PROGRESS.md)에서 도로 226,131개와 수면 16,004개를 채택하고 각각 27,417개·8개를 제외했다. Rust가 공식 디렉터리의 도로·수면 55개 카운티 목록 일치를 확인했다. 도로 원본의 실제 타일 80,579개를 압축 바이트 동일한 서부 48,312개·동부 32,267개로 나누고 두 팩과 수면 28,917개 타일을 전수 해독해 실패 0개였다. 기본 세계 모드 [Charleston](../artifacts/world-roads/wv-state/charleston-road-water-z14.png)과 [Morgantown](../artifacts/world-roads/wv-state/morgantown-road-water-z14.png) z14 도로·수면 화면도 각각 타일 실패 0개였다. 독립 위치 정확도와 경계 연결성은 미검증이다.

### 오하이오주 도로·수면 확장 — 2026-09-25

[Census 공식 ZIP 176개 감사](OH_STATE_PROGRESS.md)에서 도로선 파트 416,747개와 수면 52,216개를 채택하고, 도로 원본 레코드 33,031개와 수면 26개를 제외했다. Rust가 공식 디렉터리의 도로·수면 88개 카운티 목록 일치를 확인했다. 도로 원본의 실제 타일 155,314개를 압축 바이트 동일한 서부 71,955개·중부 40,640개·동부 42,719개로 나누고 세 팩과 수면 74,546개 타일을 전수 해독해 실패 0개였다. 기본 세계 모드 [Toledo](../artifacts/world-roads/oh-state/toledo-road-water-z14.png), [Mansfield](../artifacts/world-roads/oh-state/mansfield-road-water-z14.png), [Cleveland](../artifacts/world-roads/oh-state/cleveland-road-water-z14.png) z14 화면도 각각 타일 실패 0개였다. 독립 위치 정확도와 경계 연결성은 미검증이다.

### 워싱턴 DC 도로·수면 확장 — 2026-09-25

[Census 공식 ZIP 2개 감사](DC_PROGRESS.md)에서 도로 4,283개와 수면 141개를 채택하고 도로 52개를 제외했다. 도로 328개와 수면 208개 실제 타일을 전수 해독해 실패 0개였다. 기본 세계 모드 [DC·메릴랜드 도로·수면 z14](../artifacts/world-water/dc/dc-road-water-z14.png)에서 두 지역 출처 표기와 타일 실패 0개를 확인했다. 두 지역의 실제 도로 연결성과 독립 위치 정확도는 미검증이다.

### 프랑스 IGN 파리 도로 실증 — 2026-09-25

[BD TOPO 3.5 파리 D075 원본 감사](FR_PARIS_ROADS_PROGRESS.md)에서 차량 도로 115,758개를 채택하고 36,375개를 제외했다. 공식 7z의 내부 CRC와 원본·파생본 SHA-256, Lambert-93 좌표계, 991개 실제 타일 전수 해독에서 오류 0개를 확인했다. 기본 세계 모드 [파리 z14](../artifacts/world-roads/fr-paris/paris-world-z14.png) 캡처도 타일 실패 0개였다. 이는 파리 원천의 도로 표시 검증이며 프랑스 전국의 상세 지도나 도로의 독립 위치 정확도는 아니다.

### 프랑스 IGN 파리 수면 실증 — 2026-09-25

[BD TOPO 3.5 수면 원본 감사](FR_PARIS_WATER_PROGRESS.md)에서 537개 레코드 중 사용 중인 영구 수면 239개 폴리곤을 채택하고, 공사 중 11개·일시 수면 284개·상태 미상 3개를 제외했다. 별도 지역 팩의 실제 타일 594개를 전수 해독해 실패 0개였다. 기본 세계 모드 [파리 도로·수면 z14](../artifacts/world-water/fr-paris/paris-road-water-z14.png) 화면에 센강 일부가 표시되고 타일 실패 0개였다. 수면 경계의 독립 위치 정확도와 파리 건물·시설은 미완료다.

### 프랑스 IGN 파리 여객역 실증 — 2026-09-25

[BD TOPO 3.5 교통시설 원본 감사](FR_PARIS_STATIONS_PROGRESS.md)에서 교통시설 8,684개 중 실제 형상·운영 중·이름 조건을 만족한 여객역 99개를 채택했다. 다른 시설·임의 형상·계획/건설 중·이름 없음 8,585개는 사유와 함께 제외했다. 별도 지역 팩의 실제 타일 205개를 전수 해독해 실패 0개였다. 기본 세계 모드 [Gare du Nord 주변 z14](../artifacts/world-stations/fr-paris/paris-nord-road-water-stations-z14.png) 화면에 여객역 이름이 보이고 렌더러 집계 labels=5·타일 실패 0개였다. 파리 전체 지하철역·출입구·운행 정보와 독립 위치 정확도는 미검증이다.

### Great Britain 도로 확장 — 2026-09-24

[OS Open Roads 52개 RoadLink 격자 감사](GB_ROADS_PROGRESS.md)에서 원본 3,967,825개 레코드 중 고유 도로 ID 3,961,077개를 채택하고 격자 중복 6,748개를 제외했다. 지역 PMTiles 52개, 합계 376,893,577바이트의 비어 있지 않은 타일 480,930개를 모두 해독했으며 오류는 0개였다. 런던·에든버러·카디프·TQ–TL 경계의 기본 세계 모드 화면도 타일 오류 0개였다. 이는 공식 일반화 도로 원천의 파일·표시 검증이며 북아일랜드, 도로 연결성, 현장 위치 정확도, GB의 상세 수면·건물·시설은 포함하지 않는다.

### 캐나다 13개 지역 도로 확장 — 2026-09-24

[캐나다 NRN 원천 감사](CA_ROADS_PROGRESS.md)에서 13개 주·준주의 `ROADSEGID`가 각 지역 안에서 고유한 도로선 합계 2,548,203개를 채택하고 거절 0개를 기록했다. Ontario를 3개로 나눈 15개 PMTiles의 실제 타일 합계 1,977,578개를 전부 해독했고 오류는 0개였다. 13개 지역의 z14 기본 화면은 각각 타일 실패 0이고 캐나다 출처 표기가 보인다. NAD83(CSRS·CSRS98)·WGS84 독립 위치 비교, 지역 경계 연결성, 원본 도로망의 현장 완전성은 아직 검증하지 않았다.

Mac Metal 기본 세계 모드 캡처에서 [모나코 건물](../artifacts/world-integration/monaco-world.png), 뉴욕시 5개 카운티 팩의 [Manhattan](../artifacts/world-integration/manhattan-world.png)·[Queens](../artifacts/world-roads/nyc-queens-world-z14.png)·[Richmond](../artifacts/world-roads/nyc-richmond-world-z14.png) 도로가 실제 z14 타일로 표시됐다. 과거 [파리의 미수집 상세 영역](../artifacts/world-integration/unmapped-world.png)은 빈 중립색이었고, 현재는 [IGN 도로 팩 화면](../artifacts/world-roads/fr-paris/paris-world-z14.png)이 표시된다. [모나코 z12](../artifacts/world-integration/monaco-overview-z12.png)는 건물 타일이 시작되기 전이라 기존 z7 개략 자료를 확대하며 화면에 그 한계를 밝힌다. 여섯 캡처의 타일 실패는 각각 0개다. 최종 모나코의 동기·비동기 캡처 PNG는 SHA-256 `6477b6f1bb2cf1b12b861cdfce308e0de0a0b05e4db2aa7021989c6b73e074e2`로 일치했다. 이는 자료 선택·표시의 검증이며 현장 위치 정확도나 전 세계 커버리지 통과 판정이 아니다.

현재 목록은 파일 101개의 manifest를 시작 시 검증하고, PMTiles는 각 파일이 처음 필요할 때 연다. manifest 범위를 z5 공간 셀에 등록해 카메라·타일의 셀에 걸친 패키지만 검사하고, 실제 타일과 원천 범위의 교차를 다시 확인한다. 열린 PMTiles는 최근 사용 순서로 최대 16개를 유지하며 축출된 파일은 필요하면 다시 연다. 코드 테스트에서 매니저 생성 직후 열린 지역 PMTiles 수는 0이고, 최근 사용 파일을 제외한 축출 결과가 확인됐다. 인덱스·파일 상한 적용 전후 모나코 동기·Queens 비동기 Metal PNG는 각각 바이트 단위로 같았고 두 캡처 모두 실패 0이었다. 이 성능 검증은 당시 파일 2개에 대한 것이다. 이후 5개 파일의 세계 모드 테스트와 당시 6개 파일의 지역 목록 로드 검사, [Queens 도로·건물·수면·공원](../artifacts/world-integration/queens-four-layers-world.png) Metal 캡처에서 타일 실패 0을 확인했다. 현재 101개 목록은 로드 테스트와 GB 네 지역·캐나다 13개 지역·Delaware·Connecticut·Rhode Island·Massachusetts·New Hampshire·Vermont·Maine·Maryland·DC·Pennsylvania·West Virginia·Ohio·Paris 화면 캡처까지 검증했지만 iPhone 성능·갱신은 아직 측정되지 않았다.

Queens 건물은 정확한 z11 타일 경계를 구축 범위로 쓴다. 타일 빌더의 경계 상한을 반열린 구간으로 수정해 경계 밖 이웃 타일에 면적 없는 도형이 새지 않도록 했다. 수정 후 생성된 비어 있지 않은 건물 타일 303개와 전체 아카이브 감사에서 해독한 303개가 일치했고 실패는 0개였다. 도로·건물 중첩은 같은 z14 타일의 두 레이어가 모두 비어 있지 않음을 Rust 테스트로 확인했다. 이는 화면 결합 검증이며 서로 다른 원천의 독립 측량 정확도 증명은 아니다.

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

이 감사는 파일 해독과 표시 확인이다. 실제 현장 정확도, 도로·수계 완전성, 국가별 건물 커버리지의 독립 검증은 아니다. 원본 선이 없는 지역을 새 선으로 채우지 않는다. 기존 50m와 동아시아 10m 파일은 비교·회귀용으로 보관한다. 기본 세계 모드는 세계 110m·10m 파일을 열고 기존 34개·GB 52개·캐나다 15개, 총 101개 승인 지역 PMTiles를 필요할 때 연다. 목록의 출처 manifest는 시작 시 비공유조건 권리 판정을 통과하며, 아카이브의 지리 범위·출처 표기는 첫 사용 시 대조한다.

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
