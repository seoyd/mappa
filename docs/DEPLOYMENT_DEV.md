# v0.2.1 무료 HTTPS 개발 경로

이 문서는 **개발용** Mac의 Axum/PostgreSQL/PostGIS를 실제 기기에서 시험하는 절차다. Quick Tunnel은 영구 호스팅이 아니며 임의의 공개 URL을 발급한다. [Cloudflare 공식 문서](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/do-more-with-tunnels/trycloudflare/)도 테스트·개발 전용, SLA 없음, URL 임시, 동시 요청 제한 200개라고 명시한다. 이번 단계에서 유료 클라우드로 DB를 이전하지 않는다.

## 실행

1. 로컬 PostgreSQL/PostGIS에 **별도 테스트 DB**를 준비한다. 터널 URL에 접근 가능한 누구나 현재 인증 없는 게시 API에 요청할 수 있으므로 개인 데이터가 든 DB를 사용하지 않는다.
2. Mac에서 다음 두 프로세스를 각기 다른 터미널에서 실행한다. 포트는 실행 환경에 맞춰 바꿀 수 있다.

```sh
DATABASE_URL='postgres://.../mappa_dev' BIND_ADDR='127.0.0.1:3000' cargo run -p mappa-server
cloudflared tunnel --url http://127.0.0.1:3000 --no-autoupdate
```

3. 출력된 `https://*.trycloudflare.com/` 주소의 `/health`가 HTTP 200인지 확인한다. 앱의 **개발 HTTPS 서버 주소** 입력란에 같은 주소를 넣고 연결한다. 앱은 연결 시 한 번만 health 요청을 보내고, 주소가 바뀌면 앱 재빌드 없이 다시 연결할 수 있다. 기존 미확정 게시의 `ClientPostId`는 주소 변경 후에도 유지된다.
4. 기기에서 위치 권한을 허용하고 실제 문장을 게시한다. 같은 셀 조회 확인 후 DB에서 post/actor/client_post_id/body/cell/revision을 확인한다. 로그나 보고서에 정확한 좌표와 ActorId 값을 옮기지 않는다.
5. 시험 직후 `cloudflared`와 서버를 종료한다. 공개 URL은 소스·설정 파일에 저장하지 않는다. 앱에서 이전 URL로 재시도하면 네트워크 오류 상태가 되며 Keychain 값과 DB를 초기화하지 않는다.

Quick Tunnel은 HTTPS를 Cloudflare edge에서 종료한 뒤 Mac의 localhost로 전달한다. iOS 앱의 HTTP transport는 비-loopback HTTP를 거부한다. 앱은 ATS를 전역 비활성화하지 않는다. 현재 ActorId는 인증 수단이 아니므로 **일회성 테스트 데이터 전용**으로 사용한다.

실기기 배포에는 Xcode의 Apple 개발 팀/서명이 필요하다. [Apple의 Personal Team 안내](https://developer.apple.com/help/account/basics/about-your-developer-account)에 따르면 무료 Apple Account로 개인 기기에 설치·테스트할 수 있지만 개발 프로파일은 발급 후 7일에 만료되어 재빌드·재설치가 필요하다. 2026-09-24 이 환경에서 서명 없는 Simulator 앱은 UI를 실행했으나 Keychain은 OSStatus `-34018`을 반환했다. Keychain 검증을 완료하기 전에는 실제 기기 게시 게이트를 PASS로 기록하지 않는다.

2026-09-24 연결된 iPhone 13은 Developer Mode와 DDI가 준비됐지만, 현재 Personal Team은 등록 가능한 iPhone 한도에 도달했다. Xcode가 `No profiles for 'com.seoyd.Mappa' were found`도 보고해 이 계정으로는 아직 기기 설치가 불가능하다. Apple은 Personal Team의 기기 등록이 최대 3대이고 등록 항목이 7일 뒤 만료된다고 명시한다. 다른 사용 가능한 개발 팀을 선택하거나 기존 등록의 만료 후 다시 시도해야 한다.

## 이번 실행의 사실

2026-09-23에 `cloudflared 2026.9.1`을 설치했다. 기존 테스트 DB 대신 빈 일회성 `mappa_tunnel_gate` DB를 새로 만들고 posts=0을 확인했다. `127.0.0.1:13000` Axum `/health`는 200, Quick Tunnel HTTPS `/health`는 200과 TLS 검증 결과 0이었다. Mac에서 fake 위치를 사용한 `v021_tunnel` harness가 HTTPS → Axum → PostGIS → 같은 셀 조회에 성공했고, 동일 binary request replay 후 DB count/revision은 0→1만 증가했다. 터널·서버 종료 후 해당 DB를 삭제했다. **실제 iPhone 또는 실제 GPS는 사용하지 않았다.** 측정은 [BENCHMARK_V0_2.md](BENCHMARK_V0_2.md)에 분리해 기록한다.

```sh
MAPPA_API_BASE_URL='https://<current-quick-tunnel>/' DATABASE_URL='postgres://.../mappa_dev' cargo run -p mappa-harness --bin v021_tunnel
```

이 harness의 위치는 코드 내부의 테스트 fixture다. 위 명령은 실제 기기 검증을 대체하지 않는다.

## 향후 무료 개발 호스팅 조사 (배포 결정 아님)

| 선택지 | Rust Axum | PostgreSQL/PostGIS | 무료 자원 | idle/기동 | 지역 | 데이터 지속성 |
|---|---|---|---|---|---|---|
| 현행 Mac + Quick Tunnel | 로컬 Rust | 로컬 PostGIS | 터널 무료 | Mac/터널 실행 중에만 | Mac 위치 + Cloudflare edge | 로컬 DB가 유지되는 동안 |
| [Render](https://render.com/docs/free) | [Rust 지원](https://render.com/docs/faq) | [PostGIS 지원](https://render.com/docs/postgresql-extensions) | Web 750시간/월, 무료 DB 1GB | Web 15분 idle 후 sleep, 재기동 약 1분 | [Singapore 등 5개](https://render.com/docs/regions) | 무료 DB 30일 뒤 만료, 백업 없음 |
| [Koyeb](https://www.koyeb.com/docs/reference/instances) | Docker/Rust 서비스 가능 | [무료 PostgreSQL과 PostGIS](https://www.koyeb.com/docs/databases) | Web 0.1 vCPU/512MB, DB 활성 5시간/월·1GB | Web 1시간, DB 5분 idle 뒤 sleep | Web Frankfurt/Washington; DB Singapore 가능 | DB 서비스는 앱 수명과 분리, 무료 한도 확인 필요 |
| [Supabase](https://supabase.com/pricing) | Axum 직접 호스팅 아님 | [PostGIS 지원](https://supabase.com/features/postgres-extensions) | DB 500MB, 무료 프로젝트 2개 | [낮은 활동 후 약 7일에 pause](https://supabase.com/docs/guides/platform/free-project-pausing) | [Seoul 등](https://supabase.com/docs/guides/platform/regions) | pause 동안 복구 가능, 무료 백업 없음 |

현재 선택은 로컬 Axum/PostGIS + 임시 Quick Tunnel이다. 클라우드 옵션은 최신 공식 문서 기준의 사전조사이며, 무료 등급의 가용성·조건은 배포 시 다시 확인해야 한다.
