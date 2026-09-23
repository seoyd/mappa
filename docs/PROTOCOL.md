# Mappa binary protocol v1

모든 다중 바이트 정수는 little-endian. 문자열은 길이 뒤의 원본 UTF-8 바이트이며 정규화하지 않습니다. varint는 사용하지 않습니다. 모든 메시지는 정확히 한 프레임입니다. 요청 HTTP body 최대 65,536바이트, 프로토콜 payload 최대 2,097,152바이트. `application/x-mappa`.

## Header (16 bytes)

| Offset | Size | Field | Validation |
|---:|---:|---|---|
| 0 | 4 | ASCII `MPPA` | exact |
| 4 | 1 | version | 1 |
| 5 | 1 | kind | 1..7 |
| 6 | 2 | flags | 0 |
| 8 | 4 | request_id | echoed in response |
| 12 | 4 | payload_len | exact remaining byte count, <= 2 MiB |

## Payload

| Kind | Name | Fields in wire order |
|---:|---|---|
| 1 | CreatePostRequest | actor UUID(16), lat_e7(i32), lon_e7(i32), kind(u8), body_len(u16), body(bytes) |
| 2 | CreatePostResponse | post UUID(16), cell_id(u64), revision(u64), created_at(i64 Unix ms) |
| 3 | QueryCellsRequest | count(u8), repeated [cell_id(u64), known_revision(u64)] |
| 4 | QueryCellsResponse | count(u8), repeated cell result below |
| 5 | ErrorResponse | code(u16): 1 invalid request, 2 too large, 3 internal |
| 6 | CreatePostV2Request | actor UUID(16), client_post_id UUID(16), lat_e7(i32), lon_e7(i32), kind(u8), body_len(u16), body(bytes) |
| 7 | CreatePostV2Response | kind 2와 동일 payload; 요청 kind 6에 대한 응답 |

ErrorResponse code 4 = conflicting replay. v1 kinds 1~5의 인코딩과 프레임 버전 1은 그대로다. v2 create의 첫 요청은 HTTP 201이며 동일 내용 재전송도 저장된 원래 응답과 HTTP 201을 반환한다. 동일 `(actor_id, client_post_id)`에 다른 coordinate/kind/body를 전송하면 HTTP 409와 code 4를 반환한다. `client_post_id`는 새 logical post마다 UUIDv4를 생성하고 미확정 응답 재시도에 재사용한다.

Cell result: `cell_id(u64), revision(u64), status(u8)`. Status 0 = unchanged, no further bytes. Status 1 = complete snapshot, followed by `post_count(u16)` and that many post records. Post record: `post_id UUID(16), actor_id UUID(16), lat_e7(i32), lon_e7(i32), kind(u8), body_len(u16), body(bytes), created_at(i64 Unix ms), has_expiry(u8), [expires_at(i64 Unix ms) if has_expiry=1]`.

UUID는 RFC 4122/9562 16-byte network order. Post kind 1 = General. Body는 1..8192 UTF-8 bytes, NUL 없음, 공백만인 문자열 불가. 각 요청은 1..16개의 중복 없는 z14 cell을 가집니다. 응답은 최대 16개 cell. 알 수 없는 enum, 잘못된 좌표, 잘린 프레임, trailing bytes는 거부합니다.

## Cell ID

`[zoom:6][x:29][y:29]` (상위 비트부터). zoom 0..29, x/y 각각 `0..2^zoom-1`만 유효. 콘텐츠 요청은 zoom=14만 허용합니다. 이 packing은 version 1 프로토콜의 일부입니다. 향후 변경은 새 프로토콜 version이 필요합니다.
