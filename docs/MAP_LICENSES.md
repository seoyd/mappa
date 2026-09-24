# v0.3C 이용 조건 게이트

manifest 파서는 `APPROVED` 또는 `APPROVED_WITH_ATTRIBUTION`만 통과시키며 상업적 이용·변경·재배포가 모두 가능하다고 확인된 입력만 허용한다. `share_alike=true`, 미검토, 비상업, 변경 금지, 재배포 금지는 빌드를 거부한다. 이 플래그는 출처 검토 결과를 기록하는 것이며 법적 권리를 새로 부여하지 않는다.

| 조합 | 현재 결정 |
|---|---|
| 나주시 도로 중심선·도로면 단독 | 허용. 화면의 제공기관 표기는 manifest에서 PMTiles 메타데이터로 중복 없이 생성 |
| 나주시 도로 + 국가데이터처 SGIS 읍면동 지명 | 두 포털 자료 모두 무료·이용허락범위 제한 없음으로 표기. 원천별 provenance를 GeoDB에 유지하고 화면 제공기관 표기는 manifest에서 생성 |
| 나주시 도로 + SGIS 지명 + ESA WorldCover 2021 영구수면·수목 | ESA는 [CC BY 4.0](https://esa-worldcover.org/en/data-access)으로 상업적 이용·변경·재배포가 가능하고 출처표시가 필요하다. 각 원천은 GeoDB provenance로 구분하고 PMTiles에는 ESA의 권장 attribution 문구를 표시한다. 수목 피복을 공원으로 오기하지 않는다 |
| 나주시 도로 + 공공누리 제1유형 건물 | 원본 파일과 고지 문구를 확보한 뒤 **레이어를 분리**하여 검토 |
| 나주시 도로 + ODbL 기반 건물/도로 | 초기 canonical 통합 금지. 라이선스 전파 검토 및 OSM 없는 proof 조건과 충돌 |
| 권리 불명·비상업·변경 금지 | 통합 금지 |

원천별 최신 조건은 [나주시 데이터 포털](https://www.data.go.kr/data/15123619/fileData.do), [SGIS 자료 페이지](https://www.data.go.kr/data/15129688/fileData.do), [ESA WorldCover 이용 조건](https://esa-worldcover.org/en/data-access), [공공누리 안내](https://www.kogl.or.kr/info/publicGuide.do), [Overture 건물 안내](https://docs.overturemaps.org/guides/buildings/)에서 확인한다. `license_status`는 다운로드 가능 여부와 별도다.
