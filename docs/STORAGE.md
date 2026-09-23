# Storage

Canonical DB는 PostgreSQL + PostGIS입니다. 서버 시작 시 `migrations/0001_initial.sql`과 `migrations/0002_client_post_id.sql`을 순서대로 실행합니다. 게시글 위치 `geom`은 `geography(Point,4326)`이며 fixed-point 위경도로부터 DB 안에서 생성합니다. geography를 사용해 향후 실제 거리 질의 시 미터 단위를 쓸 수 있습니다.

일반 셀 조회는 `posts(cell_id)` 경로로 수행합니다. `(cell_id, created_at)`과 `GIST(geom)` index를 추가했습니다. 공간 거리 연산은 v0.1의 주 조회 경로에 없습니다. `cell_state(cell_id)`는 primary key입니다.

게시글 insert와 cell revision 증가는 한 transaction입니다. 조회는 repeatable-read read-only transaction으로 revision과 snapshot을 함께 읽습니다. timestamp는 Unix milliseconds BIGINT입니다. expires_at은 현재 생성 경로에서 NULL이며 만료 상태 변경 작업은 아직 없습니다.

현재 DB 연결은 프로세스당 단일 PostgreSQL connection을 mutex로 공유합니다. v0.1의 정확성 검증에는 충분하지만 높은 동시성 성능은 추후 측정 후 connection pool 도입 여부를 결정합니다.

v0.2 migration은 기존 v1 행을 보존하며 nullable `client_post_id UUID`와 `create_revision BIGINT`를 추가합니다. `client_post_id IS NOT NULL`인 행에 `(actor_id, client_post_id)` unique index를 둡니다. v2 create는 insert와 revision 증가를 transaction에서 수행합니다. 동일 요청은 저장된 행과 coordinate/kind/body를 비교한 뒤 최초 `create_revision`과 생성 시각을 돌려줍니다. 충돌 내용은 변경하지 않고 반환 오류로 처리합니다. `0002` 재실행은 `IF NOT EXISTS`를 사용합니다.
