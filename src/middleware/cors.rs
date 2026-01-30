// CORS configuration
// This is handled in main.rs via tower-http's CORS layer

use tower_http::cors::{CorsLayer, Any};
use axum::Router;

pub fn apply_cors(router: Router) -> Router {
    router.layer(
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any),
    )
}
