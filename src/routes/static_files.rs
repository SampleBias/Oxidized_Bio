//! Static File Serving
//! 
//! Serves the frontend application (built Preact/TypeScript files)
//! from the BioAgents/client/dist directory.

use axum::{
    Router,
    routing::get,
    response::{Html, IntoResponse, Response},
    http::{StatusCode, header},
};
use tower_http::services::ServeDir;
use std::path::PathBuf;
use tracing::{info, warn};

/// Get the static files directory path
fn get_static_dir() -> PathBuf {
    // Try different paths to find the frontend build
    let paths = [
        PathBuf::from("BioAgents/client/dist"),
        PathBuf::from("../BioAgents/client/dist"),
        PathBuf::from("client/dist"),
        PathBuf::from("static"),
    ];

    for path in paths {
        if path.exists() && path.is_dir() {
            info!(path = %path.display(), "Found static files directory");
            return path;
        }
    }

    // Default to first path (will 404 if not found)
    warn!("Static files directory not found, frontend may not be built");
    PathBuf::from("BioAgents/client/dist")
}

/// Create router for serving static files
pub fn router() -> Router {
    let static_dir = get_static_dir();
    
    // Create the serve directory service
    let serve_dir = ServeDir::new(&static_dir)
        .append_index_html_on_directories(true);

    Router::new()
        // Health check for frontend - serves index page
        .route("/", get(serve_index))
        // Serve static assets
        .nest_service("/assets", ServeDir::new(static_dir.join("assets")))
        // Fallback to serve directory
        .fallback_service(serve_dir)
}

/// Serve the index page
async fn serve_index() -> impl IntoResponse {
    let paths = [
        PathBuf::from("BioAgents/client/dist/index.html"),
        PathBuf::from("../BioAgents/client/dist/index.html"),
        PathBuf::from("client/dist/index.html"),
        PathBuf::from("static/index.html"),
    ];

    for path in paths {
        if let Ok(content) = tokio::fs::read_to_string(&path).await {
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                content
            ).into_response();
        }
    }

    // Return a helpful message if frontend isn't built
    let fallback_html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Oxidized Bio - API Server</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            max-width: 800px;
            margin: 0 auto;
            padding: 40px 20px;
            background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
            min-height: 100vh;
            color: #e6e6e6;
        }
        h1 { color: #00d4ff; margin-bottom: 10px; }
        h2 { color: #a0a0a0; font-weight: 400; font-size: 1.2em; }
        .status { 
            background: #0a4d3d; 
            border-radius: 8px; 
            padding: 20px; 
            margin: 30px 0;
            border-left: 4px solid #00ff9d;
        }
        .status h3 { color: #00ff9d; margin-top: 0; }
        .endpoints {
            background: #1e1e3f;
            border-radius: 8px;
            padding: 20px;
            margin: 20px 0;
        }
        code { 
            background: #2a2a4a; 
            padding: 2px 8px; 
            border-radius: 4px;
            color: #00d4ff;
        }
        pre {
            background: #0d0d1a;
            padding: 15px;
            border-radius: 6px;
            overflow-x: auto;
        }
        a { color: #00d4ff; }
        .build-note {
            background: #3d2a0a;
            border-radius: 8px;
            padding: 15px 20px;
            margin: 30px 0;
            border-left: 4px solid #ffb700;
        }
        .build-note h4 { color: #ffb700; margin-top: 0; }
    </style>
</head>
<body>
    <h1>🧬 Oxidized Bio</h1>
    <h2>AI Research Agent Backend</h2>
    
    <div class="status">
        <h3>✓ Server Running</h3>
        <p>The Rust backend API is running and ready to accept requests.</p>
    </div>
    
    <div class="build-note">
        <h4>📦 Frontend Not Built</h4>
        <p>To serve the web UI, build the frontend:</p>
        <pre>cd BioAgents
bun install
bun run build:client</pre>
    </div>
    
    <div class="endpoints">
        <h3>API Endpoints</h3>
        <ul>
            <li><code>GET /api/health</code> - Health check</li>
            <li><code>POST /api/chat</code> - Chat with the research assistant</li>
            <li><code>POST /api/deep-research/start</code> - Start deep research</li>
            <li><code>GET /api/rfc/health</code> - RFC health check</li>
        </ul>
        
        <h4>Example Chat Request:</h4>
        <pre>curl -X POST http://localhost:3000/api/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "What are the effects of rapamycin on longevity?"}'</pre>
    </div>
    
    <p style="margin-top: 40px; color: #666;">
        <a href="https://github.com/your-repo/oxidized-bio">GitHub</a> | 
        <a href="/api/health">API Health</a>
    </p>
</body>
</html>"#;

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        fallback_html.to_string()
    ).into_response()
}
