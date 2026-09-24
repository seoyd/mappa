# 프랑스 IGN 파리 D075 여객역 실증 — 2026-09-25

## 원천과 선택 기준

[IGN BD TOPO 3.5 공식 명세](https://data.geopf.fr/annexes/ressources/documentation/DC_BDTOPO_3-5.pdf)는 `Equipement de transport`의 교통시설 종류와 `FICTIF`가 임의 형상인지 표시한다고 설명한다. [공식 파리 D075 2026-06-15 배포본](https://data.geopf.fr/telechargement/resource/BDTOPO/BDTOPO_3-5_TOUSTHEMES_SHP_LAMB93_D075_2026-06-15)의 같은 7z 원본에서 교통시설 다섯 Shapefile 구성 파일을 Rust로 추출했다. 원본 SHA-256은 `dc1cdeaef2b220520faf2a05edea11af0ff2c7cc54961cb6cefd0af6e50c2414`, 사용 필드만 남긴 파생 ZIP SHA-256은 `bab9b30805741810a0f9362ad231f6a9aabc64e8bc15390355cd9c8da53ee537`이다. [Licence Ouverte 2.0](https://www.data.gouv.fr/pages/legal/licences/etalab-2.0)의 출처 표기 조건과 파일 해시는 [manifest](../data/ign_bdtopo_paris_stations.toml)에 고정했다.

원본 도형은 RGF93 / Lambert-93의 3D 폴리곤이다. 여객역 종류 중 **운영 중·`FICTIF=Non`·이름 있음** 조건에 맞는 형상의 내부 점을 Rust에서 구하고 좌표를 역투영했다. 임의 형상으로 표시된 지하철역 등을 정확한 역 위치처럼 그리지 않기 위해 제외했다.

| 단계 | 관측 사실 |
|---|---|
| 원본 | 교통시설 8,684개 레코드. 원본 `.prj`는 RGF93 / Lambert-93, `.cpg`는 UTF-8. |
| 선택 | **`FICTIF=Non`인 여객역 99개 채택** 및 피처별 출처 저장. 비여객 시설 7,994개, 임의 형상 `FICTIF=Oui` 509개, 건설 중 45개, 계획 36개, 이름 없음 1개를 [거절 기록](../artifacts/world-stations/fr-paris/stations.rejected.json.gz)에 남겼다. 조건 평가 순서에 따라 사유 하나씩 기록했다. |
| 좌표 | 실제 채택 점의 역투영 숫자 범위 `[2.170557675442588, 48.78014867003165, 2.526314528337081, 48.94589528502969]`. 파리 행정 경계나 독립 측량 정확도 값이 아니다. |
| GeoDB | 99개 역·출처, 로컬 38,369바이트, SHA-256 `93e1343446c765b232e0c102a33edb17bc54a466d182a140abc34c6143922494`. |
| 지도 팩 | [여객역 PMTiles](../artifacts/world-stations/fr-paris/stations.pmtiles) 57,586바이트, SHA-256 `a9980e972878d3a003fb2948f1774e4cd3ffa9305878feb8f429638ae8c75ef8`. z13–15의 [실제 타일 205개 전수 해독](../artifacts/world-stations/fr-paris/tile-audit.log), 실패 0개. 확대 단계별 중복 포함 `place` 297개. |
| 화면 | 기본 세계 모드 [Gare du Nord 주변 z14 도로·수면·역 합성](../artifacts/world-stations/fr-paris/paris-nord-road-water-stations-z14.png) Mac Metal 캡처에 역 이름 표시, 렌더러 집계 labels=5·타일 실패 0. |

이 팩은 **파리 전체 지하철역 목록이 아니다.** 원본의 임의 형상 509개를 표시하지 않아 많은 지하철·트램역이 비어 있다. 역 출입구·승강장·환승 연결·운행 정보는 제공하지 않는다. 역의 현장 위치 정확도, RGF93에서 WGS84로의 독립 기준점 비교, iPhone 성능도 아직 검증하지 않았다. 원본과 파생 ZIP은 로컬 빌드 입력으로 Git에 포함하지 않는다.

## 재현

```bash
cargo run --release --offline -p mappa-map-acquire --bin extract_ign_roads -- \
  data/local/ign_bdtopo_paris/BDTOPO_3-5_TOUSTHEMES_SHP_LAMB93_D075_2026-06-15.7z \
  data/local/ign_bdtopo_paris/transport.zip --transport
cargo run --release --offline -p mappa-map-data --bin make_ign_bdtopo_roads_manifest -- \
  'https://data.geopf.fr/telechargement/download/BDTOPO/BDTOPO_3-5_TOUSTHEMES_SHP_LAMB93_D075_2026-06-15/BDTOPO_3-5_TOUSTHEMES_SHP_LAMB93_D075_2026-06-15.7z' \
  data/local/ign_bdtopo_paris/BDTOPO_3-5_TOUSTHEMES_SHP_LAMB93_D075_2026-06-15.7z \
  data/local/ign_bdtopo_paris/transport.zip data/ign_bdtopo_paris_stations.toml 2026-09-25 \
  --transport
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- \
  data/ign_bdtopo_paris_stations.toml artifacts/world-stations/fr-paris/stations.mgeodb \
  --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- \
  data/ign_bdtopo_paris_stations.toml artifacts/world-stations/fr-paris/stations.mgeodb \
  artifacts/world-stations/fr-paris/stations.pmtiles
cargo run --release --offline -p mappa-map-data --bin audit_fixture -- \
  artifacts/world-stations/fr-paris/stations.pmtiles
```
