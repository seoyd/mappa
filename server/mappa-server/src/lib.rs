use axum::{
    Router,
    body::{Body, HttpBody, to_bytes},
    extract::{Request, State},
    http::{HeaderValue, StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use mappa_protocol::{
    CONTENT_TYPE as MAPPA_CONTENT_TYPE, ErrorCode, ErrorResponse, Frame, Message, decode, encode,
};
use mappa_storage::Storage;
use std::time::Instant;

const MAX_REQUEST_BYTES: usize = 64 * 1024;

pub fn router(storage: Storage) -> Router {
    Router::new()
        .route("/health", get(|| async { StatusCode::OK }))
        .route("/v1/posts", post(create_post))
        .route("/v1/cells/query", post(query_cells))
        .with_state(storage)
}

fn binary_response(status: StatusCode, bytes: Vec<u8>) -> Response {
    let mut response = (status, Body::from(bytes)).into_response();
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(MAPPA_CONTENT_TYPE));
    response
}

fn error(status: StatusCode, request_id: u32, code: ErrorCode) -> Response {
    let frame = Frame {
        request_id,
        message: Message::ErrorResponse(ErrorResponse { code }),
    };
    match encode(&frame) {
        Ok(bytes) => binary_response(status, bytes),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn read_frame(request: Request) -> Result<Frame, Box<Response>> {
    if request
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        != Some(MAPPA_CONTENT_TYPE)
    {
        return Err(Box::new(error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            0,
            ErrorCode::InvalidRequest,
        )));
    }
    let bytes = to_bytes(request.into_body(), MAX_REQUEST_BYTES)
        .await
        .map_err(|_| Box::new(error(StatusCode::PAYLOAD_TOO_LARGE, 0, ErrorCode::TooLarge)))?;
    let request_id = bytes
        .get(8..12)
        .and_then(|b| b.try_into().ok())
        .map(u32::from_le_bytes)
        .unwrap_or(0);
    decode(&bytes).map_err(|e| {
        let (status, code) = if e == mappa_protocol::ProtocolError::TooLarge {
            (StatusCode::PAYLOAD_TOO_LARGE, ErrorCode::TooLarge)
        } else {
            (StatusCode::BAD_REQUEST, ErrorCode::InvalidRequest)
        };
        Box::new(error(status, request_id, code))
    })
}

async fn create_post(State(storage): State<Storage>, request: Request) -> Response {
    let started = Instant::now();
    let frame = match read_frame(request).await {
        Ok(frame) => frame,
        Err(response) => return *response,
    };
    let request_id = frame.request_id;
    let db_started = Instant::now();
    let (result, v2) = match frame.message {
        Message::CreatePostRequest(create) => (storage.create_post(&create).await, false),
        Message::CreatePostV2Request(create) => (storage.create_post_v2(&create).await, true),
        _ => {
            return error(
                StatusCode::BAD_REQUEST,
                request_id,
                ErrorCode::InvalidRequest,
            );
        }
    };
    let db_latency_ms = db_started.elapsed().as_millis();
    let response = match result {
        Ok(created) => match encode(&Frame {
            request_id,
            message: if v2 {
                Message::CreatePostV2Response(created)
            } else {
                Message::CreatePostResponse(created)
            },
        }) {
            Ok(bytes) => binary_response(StatusCode::CREATED, bytes),
            Err(_) => error(
                StatusCode::INTERNAL_SERVER_ERROR,
                request_id,
                ErrorCode::Internal,
            ),
        },
        Err(mappa_storage::StorageError::Invalid) => error(
            StatusCode::BAD_REQUEST,
            request_id,
            ErrorCode::InvalidRequest,
        ),
        Err(mappa_storage::StorageError::Conflict) => {
            error(StatusCode::CONFLICT, request_id, ErrorCode::Conflict)
        }
        Err(_db_error) => {
            tracing::error!(
                request_id,
                error_category = "database",
                "create post failed"
            );
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                request_id,
                ErrorCode::Internal,
            )
        }
    };
    tracing::info!(request_id, message_kind = "CreatePostRequest", latency_ms = started.elapsed().as_millis(), db_latency_ms, response_bytes = response.body().size_hint().lower(), status = %response.status(), "request complete");
    response
}

async fn query_cells(State(storage): State<Storage>, request: Request) -> Response {
    let started = Instant::now();
    let frame = match read_frame(request).await {
        Ok(frame) => frame,
        Err(response) => return *response,
    };
    let request_id = frame.request_id;
    let Message::QueryCellsRequest(query) = frame.message else {
        return error(
            StatusCode::BAD_REQUEST,
            request_id,
            ErrorCode::InvalidRequest,
        );
    };
    let db_started = Instant::now();
    let result = storage.query_cells(&query.cells).await;
    let db_latency_ms = db_started.elapsed().as_millis();
    let response = match result {
        Ok(result) => match encode(&Frame {
            request_id,
            message: Message::QueryCellsResponse(result),
        }) {
            Ok(bytes) => binary_response(StatusCode::OK, bytes),
            Err(_) => error(
                StatusCode::INTERNAL_SERVER_ERROR,
                request_id,
                ErrorCode::Internal,
            ),
        },
        Err(mappa_storage::StorageError::Invalid) => error(
            StatusCode::BAD_REQUEST,
            request_id,
            ErrorCode::InvalidRequest,
        ),
        Err(_db_error) => {
            tracing::error!(
                request_id,
                error_category = "database",
                "query cells failed"
            );
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                request_id,
                ErrorCode::Internal,
            )
        }
    };
    tracing::info!(request_id, message_kind = "QueryCellsRequest", latency_ms = started.elapsed().as_millis(), db_latency_ms, response_bytes = response.body().size_hint().lower(), status = %response.status(), "request complete");
    response
}
