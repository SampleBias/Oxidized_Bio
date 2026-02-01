use anyhow::Result;
use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub llm: LLMConfig,
    pub search: SearchConfig,
    pub storage: StorageConfig,
    pub auth: AuthConfig,
    pub payment: PaymentConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub host: String,
    pub cors_allowed_origins: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LLMConfig {
    pub openai_api_key: String,
    pub anthropic_api_key: String,
    pub google_api_key: String,
    pub openrouter_api_key: String,
    pub groq_api_key: String,
    pub default_provider: String,
    pub default_model: String,
}

impl LLMConfig {
    pub fn active_api_key(&self) -> Option<String> {
        let key = match self.default_provider.as_str() {
            "openai" => &self.openai_api_key,
            "anthropic" => &self.anthropic_api_key,
            "google" => &self.google_api_key,
            "openrouter" => &self.openrouter_api_key,
            "groq" => &self.groq_api_key,
            _ => "",
        };
        if key.is_empty() {
            None
        } else {
            Some(key.to_string())
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchConfig {
    /// SerpAPI key for Google Scholar and Google Light searches
    pub serpapi_key: String,
    /// Enable Google Scholar search (primary)
    pub scholar_enabled: bool,
    /// Enable Google Light search (secondary/fallback)
    pub light_enabled: bool,
    /// Maximum results per search
    pub max_results: usize,
}

impl SearchConfig {
    /// Check if SerpAPI is configured and available
    pub fn serpapi_available(&self) -> bool {
        !self.serpapi_key.is_empty() && (self.scholar_enabled || self.light_enabled)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    pub provider: String,
    pub s3_bucket: String,
    pub s3_region: String,
    pub s3_access_key_id: Option<String>,
    pub s3_secret_access_key: Option<String>,
    pub s3_endpoint: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub secret: String,
    pub mode: String,
    pub max_jwt_expiration: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaymentConfig {
    pub x402_enabled: bool,
    pub b402_enabled: bool,
    pub x402_environment: String,
    pub x402_payment_address: Option<String>,
    pub x402_network: String,
    pub cdp_api_key_id: Option<String>,
    pub cdp_api_key_secret: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        Ok(Self {
            server: ServerConfig {
                port: env::var("PORT")
                    .unwrap_or_else(|_| "3000".to_string())
                    .parse()?,
                host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                cors_allowed_origins: env::var("ALLOWED_ORIGINS")
                    .unwrap_or_else(|_| "http://localhost:3000,http://localhost:5173".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
            },
            database: DatabaseConfig {
                // DATABASE_URL is optional - the app can run without a database
                // (e.g., TUI mode, or server mode with just API key validation)
                url: env::var("DATABASE_URL")
                    .unwrap_or_else(|_| String::new()),
                max_connections: env::var("DB_MAX_CONNECTIONS")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()?,
                min_connections: env::var("DB_MIN_CONNECTIONS")
                    .unwrap_or_else(|_| "1".to_string())
                    .parse()?,
            },
            redis: RedisConfig {
                url: env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string()),
                enabled: env::var("USE_JOB_QUEUE")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()?,
            },
            llm: LLMConfig {
                openai_api_key: env::var("OPENAI_API_KEY").unwrap_or_default(),
                anthropic_api_key: env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
                google_api_key: env::var("GOOGLE_API_KEY").unwrap_or_default(),
                openrouter_api_key: env::var("OPENROUTER_API_KEY").unwrap_or_default(),
                groq_api_key: env::var("GROQ_API_KEY").unwrap_or_default(),
                default_provider: env::var("REPLY_LLM_PROVIDER").unwrap_or_else(|_| "openai".to_string()),
                default_model: env::var("REPLY_LLM_MODEL").unwrap_or_else(|_| "gpt-4".to_string()),
            },
            search: SearchConfig {
                serpapi_key: env::var("SERPAPI_KEY").unwrap_or_default(),
                scholar_enabled: env::var("SERPAPI_SCHOLAR_ENABLED")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                light_enabled: env::var("SERPAPI_LIGHT_ENABLED")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                max_results: env::var("SERPAPI_MAX_RESULTS")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .unwrap_or(10),
            },
            storage: StorageConfig {
                provider: env::var("STORAGE_PROVIDER").unwrap_or_else(|_| "s3".to_string()),
                s3_bucket: env::var("S3_BUCKET").unwrap_or_default(),
                s3_region: env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
                s3_access_key_id: env::var("AWS_ACCESS_KEY_ID").ok(),
                s3_secret_access_key: env::var("AWS_SECRET_ACCESS_KEY").ok(),
                s3_endpoint: env::var("S3_ENDPOINT").ok(),
            },
            auth: AuthConfig {
                // BIOAGENTS_SECRET is optional - defaults to a random value if not set
                // For production, this should always be explicitly set
                secret: env::var("BIOAGENTS_SECRET")
                    .unwrap_or_else(|_| {
                        // Generate a random secret for dev/local use
                        use std::collections::hash_map::RandomState;
                        use std::hash::{BuildHasher, Hasher};
                        let random = RandomState::new().build_hasher().finish();
                        format!("dev-secret-{:016x}", random)
                    }),
                mode: env::var("AUTH_MODE").unwrap_or_else(|_| "none".to_string()),
                max_jwt_expiration: env::var("MAX_JWT_EXPIRATION")
                    .unwrap_or_else(|_| "3600".to_string())
                    .parse()?,
            },
            payment: PaymentConfig {
                x402_enabled: env::var("X402_ENABLED")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()?,
                b402_enabled: env::var("B402_ENABLED")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()?,
                x402_environment: env::var("X402_ENVIRONMENT").unwrap_or_else(|_| "testnet".to_string()),
                x402_payment_address: env::var("X402_PAYMENT_ADDRESS").ok(),
                x402_network: env::var("X402_NETWORK").unwrap_or_else(|_| "base-sepolia".to_string()),
                cdp_api_key_id: env::var("CDP_API_KEY_ID").ok(),
                cdp_api_key_secret: env::var("CDP_API_KEY_SECRET").ok(),
            },
        })
    }
}
