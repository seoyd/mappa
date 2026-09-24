# Offline zoom LOD slice — 2026-09-24 KST

Mac mini Apple M4 / Metal에서 최종 OSM 해안·수면·녹지 및 도로 등급 타일로 `/usr/bin/time -l target/debug/mappa-map-demo --zoom-compare`를 한 번 실행했다. 모든 출력은 1200×720이다. z4.4–z7.4는 (127.5°E, 37.5°N), z8.4–z13.4는 OSM의 서울 도시 점 (126.978291°E, 37.566679°N)을 중심으로 했다. `load_ms`는 로컬 타일 읽기·해독·geometry 준비·GPU 업로드를 포함하고 PNG readback·표시는 제외한다. CPU/GPU 수치는 열 화면을 같은 프로세스에서 순서대로 처리한 후의 누적 cache 값이다. `overlay`는 화면에 채택된 지도 이름뿐 아니라 범례·OSM 출처 문구도 센다.

| 화면 | 카메라 zoom | 파일 / 타일 z | overlay | load | CPU cache | GPU buffer | 해독 실패 |
|---|---:|---|---:|---:|---:|---:|---:|
| `overview.png` | 4.4 | 세계 Natural Earth / z4 | 3 | 5.399 ms | 47,852 B | 279,512 B | 0 |
| `coast.png` | 5.4 | 동아시아 Natural Earth / z5 | 2 | 112.779 ms | 649,816 B | 10,437,096 B | 0 |
| `roads.png` | 6.4 | 동아시아 Natural Earth / z6 | 9 | 66.077 ms | 1,705,941 B | 18,053,244 B | 0 |
| `close.png` | 7.4 | 동아시아 Natural Earth / z7 | 3 | 39.658 ms | 2,036,895 B | 22,407,628 B | 0 |
| `korea-network.png` | 8.4 | 한국 OSM / z8 | 23 | 3,386.072 ms | 26,168,228 B | 94,214,484 B | 0 |
| `seoul-approach.png` | 9.4 | 한국 OSM / z9 | 37 | 2,704.521 ms | 50,121,656 B | 124,186,788 B | 0 |
| `city.png` | 10.4 | 수도권 OSM / z10 | 17 | 2,022.766 ms | 65,935,159 B | 129,963,704 B | 0 |
| `stations.png` | 11.4 | 수도권 OSM / z11 | 38 | 1,035.251 ms | 66,902,795 B | 129,246,536 B | 0 |
| `civic.png` | 12.4 | 수도권 OSM / z12 | 39 | 547.549 ms | 65,479,405 B | 130,648,944 B | 0 |
| `civic-close.png` | 13.4 | 수도권 OSM / z12 확대 | 39 | 0.022 ms | 65,479,405 B | 130,648,944 B | 0 |

열 화면 모두 바다처럼 소스에 의도적으로 없는 타일을 제외한 visible tile 누락 검사에 통과했다. 128 MiB GPU / 64 MiB CPU cache를 사용했다. `/usr/bin/time -l`은 maximum resident set size 272,154,624 B와 peak memory footprint 498,877,352 B를 보고했다. 별도의 타일 전수 감사에서 한국 173개와 수도권 840개 모두 해독에 성공했다. 도시·역·공공기관 이름은 실제 소스 이름이며, 충돌 선별 때문에 모든 개체의 이름을 한 화면에 표기하지는 않는다. 지역 소스 전환이 일어나는 z8–z10의 첫 정지 화면 로딩은 2–3초대로 아직 느리다.

이 수치는 macOS 데스크톱의 한 번의 정지 화면 측정이다. 실제 pan/zoom 제스처 p95/p99, iPhone 메모리와 프레임 시간, 넓은 지역에서 반복 이동할 때의 cache 안정성은 아직 측정하지 않았다. 현재 오프스크린 로딩만으로 부드러운 모바일 사용성을 판정할 수 없다.
