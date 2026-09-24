# MappaGeoDB v1

현재 파일은 `MAPPAGEO` magic, schema version, 레코드 수와 섹션 오프셋, 길이 제한된 bincode feature 레코드, 56바이트/건 공간 인덱스, source/provenance 메타데이터, 전체 SHA-256 footer로 구성된다. f64 WGS84 좌표를 보존한다. 열 때 길이·오프셋·checksum을 확인하고 R-tree 공간 질의를 지원한다. 현재 파일은 3,827개 도로 중심선, 4,477개 유효 도로면 폴리곤, 14개 공식 읍면동 위치 레이블, ESA WorldCover 2021 영구수면 79개 polygon과 수목 피복 2,609개 polygon을 담고 있다. 총 11,006개 feature다. 제품 런타임은 이 파일을 열지 않고 742,590바이트 PMTiles만 읽는다.

Canonical feature ID는 source ID와 source feature ID를 SHA-256으로 조합한 u128이며 원본 `gid`/`ADM_CD` 자체가 아니다. 종류·geometry·bbox·importance·zoom 범위·revision·선택적 이름을 보관한다. 어댑터가 원본 필드를 읽고 표준 road 종류, `PlaceDistrict`, `Water`, `Vegetation`으로 바꾼다. `Vegetation`은 WorldCover의 수목 피복이며 공원 경계가 아니다. 타일 빌더는 GeoDB만 읽는다.

도로면 원본 7,667건 중 17건은 invalid polygon으로 [거부 목록](../artifacts/map-v0.3c/naju-roads.rejected.json)에 기록했고 수리하지 않았다. 그중 proof bbox와 겹치는 유효 polygon 4,477개가 DB에 들어갔다.

**미구현:** 여러 독립 원천의 같은 객체 matching/fusion, 수정 revision 체인과 tombstone, 문자열 테이블, geometry delta 압축, 건물·공식 수계·철도·공원 경계 어댑터. 현재 `revision=1`은 초기 값이지 업데이트 이력을 구현했다는 뜻이 아니다. 2개 이상의 합법적 동일 객체 원천이 없으므로 fusion 정확도를 측정할 수 없다.
