# 프랑스 IGN 파리 D075 도로 실증 — 2026-09-25

## 원천·권리

[IGN BD TOPO 3.5](https://www.data.gouv.fr/datasets/bd-topo-r)는 프랑스 지리정보·산림연구소가 만든 공식 벡터 자료이며 [Licence Ouverte 2.0](https://www.data.gouv.fr/pages/legal/licences/etalab-2.0)으로 게시된다. 상업적 재사용·변형·재배포가 가능하고 출처 표기가 필요하며 공유조건은 없다. [2026-06-15 파리 D075 Shapefile 배포 목록](https://data.geopf.fr/telechargement/resource/BDTOPO/BDTOPO_3-5_TOUSTHEMES_SHP_LAMB93_D075_2026-06-15)에서 7z 원본 142,874,290바이트를 확인했다. 파일 전체 GET은 이 환경에서 HTTP 500을 반환했으므로 Rust 범위 다운로더로 69개 조각을 받아 각 `Content-Range`와 길이를 확인했다.

원본 SHA-256은 `dc1cdeaef2b220520faf2a05edea11af0ff2c7cc54961cb6cefd0af6e50c2414`이다. 배포 목록에는 이 판의 별도 게시 해시가 없어 게시 바이트 수·응답 범위·7z 내부 CRC 검사를 통과한 사실만 확인했다. Rust 추출기는 `TRANSPORT/TRONCON_DE_ROUTE`의 `.shp`·`.shx`·`.dbf`·`.prj`·`.cpg` 다섯 파일만 선택하고, 사용하지 않는 날짜 필드의 원본 `00000000` 때문에 도로 처리에 필요한 DBF 필드 8개를 원본 레코드 순서대로 복사한다. 파생 도로 ZIP SHA-256은 `26a66d6fcf8cfeeb97852af2326799ec9c392c08855ca7e2398b3f465f97e361`이다. 원본 7z와 파생 ZIP은 로컬 빌드 입력이며 Git 저장소에 넣지 않는다. 두 해시와 URL은 [manifest](../data/ign_bdtopo_paris_roads.toml)에 고정했다.

## 실제 처리 결과

| 단계 | 관측 사실 |
|---|---|
| 원본 | 도로 DBF 152,133개 레코드. 원본 `.prj`는 RGF93 / Lambert-93와 GRS80 타원체·투영 매개변수를 기록한다. `.cpg`는 UTF-8이다. |
| 선택 | **차량 도로 115,758개 채택**. 원본의 `ETAT=En service`, `FICTIF=Non`, 차량 접근 허용 여부와 `NATURE`·`IMPORTANCE`를 판정한다. |
| 제외 | **36,375개**: 차량 접근 `Physiquement impossible` 35,617개, `En projet` 497개, `En construction` 171개, `FICTIF=Oui` 90개. [압축 거절 기록](../artifacts/world-roads/fr-paris/roads.rejected.json.gz)과 [원천 수](../artifacts/world-roads/fr-paris/source-counts.csv)를 보관한다. 조건 평가 순서에 따라 사유 하나씩 기록했다. |
| 좌표 | Rust Lambert-93 역투영을 PROJ의 독립 계산값 3점과 비교한 단위 테스트가 통과했다. 채택된 도로의 변환 후 숫자 범위 `[2.153918089573465, 48.76481276145325, 2.5426011395319663, 48.94948403965513]`. 이는 채택된 원천 형상의 범위로 파리 행정 경계가 아니다. |
| GeoDB | 115,758개 피처·피처별 출처, 로컬 49,505,967바이트, SHA-256 `21b3700d93211a4edaef559ff5e02debc8880643b4767e497688a7177cc2e2e8`. |
| 지도 팩 | [Paris PMTiles](../artifacts/world-roads/fr-paris/roads.pmtiles) 6,880,025바이트, SHA-256 `0b8e4e07ca7f5b6bb3a9a7e305f5c7743aab5487e8bd64759f88b10f6b3f72c7`. z10–15 실제 타일 [991개 전수 해독](../artifacts/world-roads/fr-paris/tile-audit.log), 부재 0·실패 0. |
| 화면 | 기본 세계 모드 [파리 z14 Metal 캡처](../artifacts/world-roads/fr-paris/paris-world-z14.png) 타일 실패 0, IGN 출처 표기 확인. |

도로 중심선의 파일·타일·화면을 검증한 결과다. 도로 연결성·모든 실제 도로의 완전성·현장 위치 정확도·RGF93에서 WGS84로의 독립 기준점 비교는 통과 판정하지 않았다. IGN의 [BD TOPO 3.5 명세](https://data.geopf.fr/annexes/ressources/documentation/DC_BDTOPO_3-5.pdf)는 도로의 성격과 취득 방법에 따른 정밀도 정보를 별도로 정의한다. 이번 팩에는 파리 상세 수면·건물·시설이 아직 없으며 iPhone 성능도 측정하지 않았다.

## 재현

```bash
cargo run --release -p mappa-map-acquire --bin ranged -- \
  'https://data.geopf.fr/telechargement/download/BDTOPO/BDTOPO_3-5_TOUSTHEMES_SHP_LAMB93_D075_2026-06-15/BDTOPO_3-5_TOUSTHEMES_SHP_LAMB93_D075_2026-06-15.7z' \
  data/local/ign_bdtopo_paris/BDTOPO_3-5_TOUSTHEMES_SHP_LAMB93_D075_2026-06-15.7z 142874290
cargo run --release --offline -p mappa-map-acquire --bin extract_ign_roads -- \
  data/local/ign_bdtopo_paris/BDTOPO_3-5_TOUSTHEMES_SHP_LAMB93_D075_2026-06-15.7z \
  data/local/ign_bdtopo_paris/road.zip
cargo run --release --offline -p mappa-map-data --bin make_ign_bdtopo_roads_manifest -- \
  'https://data.geopf.fr/telechargement/download/BDTOPO/BDTOPO_3-5_TOUSTHEMES_SHP_LAMB93_D075_2026-06-15/BDTOPO_3-5_TOUSTHEMES_SHP_LAMB93_D075_2026-06-15.7z' \
  data/local/ign_bdtopo_paris/BDTOPO_3-5_TOUSTHEMES_SHP_LAMB93_D075_2026-06-15.7z \
  data/local/ign_bdtopo_paris/road.zip data/ign_bdtopo_paris_roads.toml 2026-09-25
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- \
  data/ign_bdtopo_paris_roads.toml artifacts/world-roads/fr-paris/roads.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- \
  data/ign_bdtopo_paris_roads.toml artifacts/world-roads/fr-paris/roads.mgeodb \
  artifacts/world-roads/fr-paris/roads.pmtiles
cargo run --release --offline -p mappa-map-data --bin audit_fixture -- \
  artifacts/world-roads/fr-paris/roads.pmtiles
```
