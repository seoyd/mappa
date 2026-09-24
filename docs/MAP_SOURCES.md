# 지도 원천 목록 (2026-09-24)

빌드에 허용한 입력은 [`data/sources.toml`](../data/sources.toml)의 나주시 도로 중심선·도로면과 국가데이터처 SGIS 읍면동 행정경계다. 도로 2개 레이어는 동일 공식 ZIP에서 왔다. 파일 SHA-256, 원천 버전, 좌표계 메모, 어댑터 버전, 이용 조건을 잠근다. 도로 원본의 직접 파일 URL은 보존되지 않아 해당 manifest의 `download_page_url`은 안내·다운로드 페이지다. 내려받은 도로 ZIP과 변환 GeoJSON의 해시는 [기존 기록](PUBLIC_ROADS_PILOT.md)에 있다.

| 후보 | 범위·형식 | 권리/접근 판정 | 이번 실증 |
|---|---|---|---|
| [나주시 도로(차도)](https://www.data.go.kr/data/15123619/fileData.do) | 나주, 2023-01-01 SHP → WGS84 GeoJSON | 포털 표기 무료·이용허락범위 제한 없음. 원본 `.prj`가 없어 EPSG:5186 가정 변환, 기관 확인 대기 | `APPROVED`, 도로 중심선·도로면 2개 레이어 입력 |
| [국토교통부 GIS건물통합정보](https://www.data.go.kr/data/15083092/fileData.do) | 전국 건물 SHP, 업데이트되는 원천 | [공공누리 제1유형](https://www.kogl.or.kr/info/publicGuide.do) 출처표시. [브이월드 다운로드](https://www.vworld.kr/dtmk/dtmk_ntads_s002.do?svcCde=NA&dsId=18)에서 2026-09-09 전남광주통합특별시 전체 SHP 258MB 및 배포 좌표계 EPSG:5186을 확인. 다운로드 클릭 시 “로그인 후 이용해주세요” | `APPROVED_WITH_ATTRIBUTION` 후보이나 파일 미확보, 미입력 |
| [국토교통부 건축물연령정보](https://www.data.go.kr/data/15048122/fileData.do) | 전국 5,720,799행 SHP, 2024-02-14 자료 | 포털 표기 무료·이용허락범위 제한 없음. 실제 파일은 [브이월드](https://www.vworld.kr/dtmk/dtmk_ntads_s002.do?svcCde=NA&dsId=1) 제공이며 다운로드 가능 여부·건물 형상·나주 범위는 아직 확인하지 못함 | 접근·원본 미확인, 미입력 |
| [국가데이터처 SGIS 행정구역 통계 및 경계](https://www.data.go.kr/data/15129688/fileData.do) | 전국 2025년 2분기 행정경계 SHP와 2024년 통계 CSV | 무료·이용허락범위 제한 없음. 공식 ZIP SHA-256 `f1cf0f9de453ac7eaacb273f39cee52851183372b9ddfda428a967c3a670b2c6`, 읍면동 `.prj`는 EPSG:5179. bbox와 겹친 14개 polygon을 WGS84 GeoJSON으로 변환; 변환본 SHA-256은 manifest에 잠금. ZIP 269,032,521B는 Git 제외 캐시에 보관 | `APPROVED`, 실제 읍면동 이름의 위치 레이블 14개 입력. 행정경계 선 자체는 미표시 |
| [Microsoft Global ML Building Footprints](https://github.com/microsoft/GlobalMLBuildingFootprints) | 글로벌 타일별 건물 | [CDLA Permissive 2.0](https://cdla.dev/permissive-2-0/). 확인한 2026-08-13 공식 인덱스에 나주/안산을 덮는 L9 타일 항목 없음 | 지역 coverage 부족, 미입력 |
| [Overture Buildings](https://docs.overturemaps.org/guides/buildings/) | 글로벌 GeoParquet | 건물 테마 전체가 OSM 포함 ODbL로 배포됨 | OSM 없는 proof에서 제외 |
| [Zenodo 한국 건물 후보](https://zenodo.org/records/20653796) | 지역별 건물 | CC-BY 표시는 있으나 상류 도로명주소 DB 재배포 권리 검토 필요 | `NEEDS_REVIEW`, 미입력 |

건물·수면·철도·공원·역·공공기관은 이 proof의 승인·확보된 입력이 0개다. 지명은 SGIS 읍면동 14개만 확보했다. 원천 공백을 추정 지형으로 채우지 않는다. 다운로드 캐시는 `.mappa-map-cache/`에 두고 Git에서 제외한다. SGIS 파생본 재생성은 공식 ZIP의 `bnd_dong_00_2025_2Q` SHP에 대해 `ogr2ogr -f GeoJSON -t_srs EPSG:4326 -spat_srs EPSG:4326 -spat 126.69 34.99 126.78 35.08 -lco RFC7946=YES`를 사용한다. GIS 변환 도구는 빌드 전 원본 준비에만 쓰며 앱과 MappaGeoDB 생성·타일 생성은 Rust다.
