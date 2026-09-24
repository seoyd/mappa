# 오프라인 지도 팩 배포 게이트 — 2026-09-25

## 확인된 규모와 문제

노스캐롤라이나주 추가 후 지역 카탈로그 세 개에는 도로·수면 등 **111개 PMTiles 팩, 2,012,392,332바이트**가 등록돼 있다. 이번 변경을 포함하면 Git이 추적하는 모든 PMTiles는 **122개, 2,056,151,915바이트**다. 추가 전 로컬 `.git` 디렉터리는 측정 시 약 **1.8GB**였다. 이는 지도 내용의 세계 범위 완성도와 무관한 저장소 규모다.

[GitHub의 일반 Git 파일 지침](https://docs.github.com/en/repositories/working-with-files/managing-large-files/about-large-files-on-github)은 50MiB 이상 경고, 100MiB 초과 차단, 저장소 1GB 미만 권장 및 5GB 미만 강력 권장을 명시한다. 따라서 지역을 늘릴 때마다 모든 PMTiles 바이너리를 Git 이력에 쌓는 방식은 전 세계 상세 지도에 맞지 않는다. 과거 이력을 자동으로 다시 쓰지 않았고, 기존 팩도 아직 Git에서 제거하지 않았다.

## 현재 구현한 로컬 게이트

[재고표](../data/map_pack_inventory.toml)는 세 [지역 카탈로그](../assets/map/regional_packs.toml), [GB 카탈로그](../assets/map/gb_regional_packs.toml), [캐나다 카탈로그](../assets/map/ca_regional_packs.toml)의 **모든 등록 팩**에 대해 저장소 상대 경로, 출처 manifest 경로·SHA-256, 팩 바이트 수·SHA-256을 고정한다. [Rust 도구](../crates/mappa-map-acquire/src/bin/map_pack_bundle.rs)는 카탈로그 누락·중복, manifest 변조, 팩 크기·해시 불일치를 거절한다. 소스 디렉터리나 HTTPS 정적 파일에서 설치할 때 파일을 임시 경로에 받은 뒤 전체 크기와 SHA-256을 확인하고 원자적으로 배치한다. 이미 있는 파일도 검증한다. HTTPS 파일명은 `<팩 SHA-256>.pmtiles`다.

```sh
cargo run --offline -p mappa-map-acquire --bin map_pack_bundle -- inventory . data/map_pack_inventory.toml
cargo run --offline -p mappa-map-acquire --bin map_pack_bundle -- verify . data/map_pack_inventory.toml
cargo run --offline -p mappa-map-acquire --bin map_pack_bundle -- stage . data/map_pack_inventory.toml /tmp/mappa-pack-stage
cargo run --offline -p mappa-map-acquire --bin map_pack_bundle -- install-dir . data/map_pack_inventory.toml /tmp/mappa-pack-stage
cargo run --offline -p mappa-map-acquire --bin map_pack_bundle -- install-url . data/map_pack_inventory.toml https://github.com/seoyd/mappa/releases/download/TAG/
```

마지막 명령의 `TAG`는 **예시 자리표시자**다. 현재 GitHub 릴리스 목록 조회 결과는 비어 있었고, 원격 지도 팩은 아직 게시·다운로드 검증하지 않았다. 오프라인 지도 런타임은 네트워크 요청을 하지 않는다. 원격 설치 명령은 사용자가 별도로 실행하는 설치·갱신 단계용이다.

초기 106개 팩 전체의 해시 대조가 통과했고, `stage`가 SHA-256 파일명으로 106개 자산을 만들었다. 카탈로그 세 개와 해당 manifest 106개만 있는 별도 빈 디렉터리에 이 자산을 `install-dir`로 복원한 결과 `installed_packs=106 verified_packs=106`이었다. 노스캐롤라이나주 다섯 팩을 추가한 뒤 재고표를 다시 생성하고 전체 111개를 `stage`로 검증한 결과 `staged_assets=5 inventory_packs=111`이었다. 111개 전체의 새 빈 디렉터리 복원은 아직 반복하지 않았다. 이 검증은 로컬 정적 파일 결과이며 GitHub 다운로드 성공, 모바일 설치 크기·속도, 실제 지도 위치 정확도를 뜻하지 않는다.

## 정적 배포 후보와 아직 남은 일

[GitHub Releases 문서](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)는 릴리스당 자산 최대 1,000개, 파일당 2GiB 미만, 총 릴리스 크기 및 전송량 제한 없음이라고 설명한다. 111개 팩은 현재 자산 수 한도 안에 있고 각 파일도 2GiB 미만이다. 이 조건은 현재 게시 정책이며 영구적인 무상 제공 보장은 아니다. [Git LFS](https://docs.github.com/en/billing/concepts/product-billing/git-lfs)는 무료 할당량 이후 과금 또는 차단이 가능하므로 기본 배포 경로로 선택하지 않는다.

다음 게이트는 실제 정적 릴리스 자산 게시, 새 체크아웃에서 설치 및 전체 해시 검증, 카탈로그가 모든 팩을 열 수 있는지 확인, 새 팩을 Git에서 제외하는 전환이다. **현재 이 전환은 완료되지 않았다.** 기존 등록 파일은 아직 Git이 제공하므로 지금의 체크아웃 재현성은 유지된다. 장기적으로 다운로드 저장소가 바뀌면 저장소에 고정한 해시를 만족하는 미러로 교체할 수 있다. 팩의 원천 라이선스와 출처 manifest 게이트는 배포 방식과 별개로 유지된다.
