// Authentication middleware stub
// TODO: Implement full JWT authentication

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};

pub async fn auth_middleware(
    req: Request,
    next: Next,
) -> Result<Response, axum::http::StatusCode> {
    // Placeholder: would verify JWT token
    Ok(next.run(req).await)
}

pub fn verify_jwt(token: &str) -> Result<(), String> {
    // Placeholder: would verify JWT signature and claims
    Ok(())
}
