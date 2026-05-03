//! Phase 2 placeholder — implementation lands in Plan 04 Task 3.

pub fn handle<R: tauri::Runtime>(
    _ctx: tauri::UriSchemeContext<'_, R>,
    _request: http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
) {
    // No-op stub. Real implementation lands in Plan 04 Task 3.
    let _ = responder.respond(
        http::Response::builder()
            .status(404)
            .body(Vec::new())
            .unwrap(),
    );
}
