# ESA WorldCover 2021 나주 입력 재생성

이 실증은 [ESA WorldCover 2021 v200](https://esa-worldcover.org/en/data-access)의 공개 10m COG에서 나주 proof bbox `[126.69, 34.99, 126.78, 35.08]`만 추출했다. 원본은 `N33E126` 타일이며 EPSG:4326, CC BY 4.0이다. ESA 공식 클래스 `80`은 **permanent water bodies**, `10`은 **tree cover**다. 수목 피복은 법정 공원 경계가 아니다. 자료 기준 연도는 2021년이고 수계 역시 현재 측량 경계가 아니다.

원본 URL: `https://esa-worldcover.s3.eu-central-1.amazonaws.com/v200/2021/map/ESA_WorldCover_10m_2021_v200_N33E126_Map.tif`

GDAL 3.13.3에서 원본 COG의 HTTP range 요청으로 다음을 실행했다. 빌드 시 인터넷이나 외부 API는 필요하지 않다. 완전한 3° 원본 COG는 저장하지 않았으며, 버전이 지정된 원본 URL과 추출 범위·GDAL 버전을 기록한다.

```sh
gdal_translate -q -projwin 126.69 35.08 126.78 34.99 \
  '/vsicurl/https://esa-worldcover.s3.eu-central-1.amazonaws.com/v200/2021/map/ESA_WorldCover_10m_2021_v200_N33E126_Map.tif' \
  assets/map/source/public/esa_worldcover_naju_2021_crop.tif

gdal_calc.py -A assets/map/source/public/esa_worldcover_naju_2021_crop.tif \
  --calc='A==80' --type=Byte --NoDataValue=0 \
  --outfile=/tmp/mappa-wc-water-mask.tif --overwrite --quiet
gdal_polygonize.py -q -overwrite -mask /tmp/mappa-wc-water-mask.tif \
  /tmp/mappa-wc-water-mask.tif \
  assets/map/source/public/naju_worldcover_water.geojson water DN

gdal_calc.py -A assets/map/source/public/esa_worldcover_naju_2021_crop.tif \
  --calc='A==10' --type=Byte --NoDataValue=0 \
  --outfile=/tmp/mappa-wc-tree-mask.tif --overwrite --quiet
gdal_polygonize.py -q -overwrite -mask /tmp/mappa-wc-tree-mask.tif \
  /tmp/mappa-wc-tree-mask.tif \
  assets/map/source/public/naju_worldcover_tree.geojson tree DN
```

| 입력 | SHA-256 | 크기/개수 |
|---|---|---:|
| 추출 COG | `091a3b1aa947841a0167d374441cf90d8bfca88192c8d42187bf780520653f93` | 1080×1080 픽셀, 1,170,362 B |
| 물 GeoJSON | `984c5b40caf1c428a31c0d8e53458e78400909dce57a4c4349c0c7e36f13ed5f` | 79 polygon |
| 수목 GeoJSON | `9ca5f615da15fe34991ff4ba19dd63bffc8689f3400052119d45c94ab900107b` | 2,609 polygon |

커밋된 crop에서 두 GeoJSON을 다시 생성했을 때 SHA-256이 위 값과 정확히 일치했다. [`data/sources.toml`](../data/sources.toml)은 crop과 GeoJSON의 해시를 모두 검사한다. 이후 GeoJSON은 Rust 어댑터를 거쳐 MappaGeoDB에 들어가고, Rust 타일 빌더가 로컬 PMTiles를 만든다. 앱 실행 시 ESA 서버 호출은 없다. 제품 attribution에는 ESA 권장 문구를 포함한다.

WorldCover의 전체 클래스 분류 정확도는 ESA가 76.7%로 보고한다. 이 숫자는 나주 물·수목 개별 클래스 정확도가 아니다. 픽셀 모서리의 계단형 경계는 10m 분류 격자의 실제 한계이며 보간한 측량 경계처럼 표시하지 않는다.
