use std::sync::OnceLock;

static ADMIN_TOKEN: OnceLock<String> = OnceLock::new();
static DEFAULT_CANVAS_WIDTH: OnceLock<usize> = OnceLock::new();
static DEFAULT_CANVAS_HEIGHT: OnceLock<usize> = OnceLock::new();
static DEFAULT_SNAPSHOT_INTERVAL: OnceLock<usize> = OnceLock::new();
static DATABASE_PATH: OnceLock<String> = OnceLock::new();
static RATE_LIMIT_TOKENS: OnceLock<f64> = OnceLock::new();
static RATE_LIMIT_REFILL_RATE_MS: OnceLock<f64> = OnceLock::new();

pub fn init() {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();
    
    let token = std::env::var("ADMIN_TOKEN")
        .expect("ADMIN_TOKEN must be set in .env file");
    
    let width = std::env::var("DEFAULT_CANVAS_WIDTH")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(256);
    
    let height = std::env::var("DEFAULT_CANVAS_HEIGHT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(256);
    
    let snapshot_interval = std::env::var("DEFAULT_SNAPSHOT_INTERVAL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000);
    
    let db_path = std::env::var("DATABASE_PATH")
        .unwrap_or_else(|_| "/data/place.db".to_string());

    let rate_limit_tokens = std::env::var("RATE_LIMIT_TOKENS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(50.0);
        
    let rate_limit_refill_rate_ms = std::env::var("RATE_LIMIT_REFILL_RATE_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100.0);
    
    RATE_LIMIT_TOKENS.set(rate_limit_tokens).expect("Failed to set RATE_LIMIT_TOKENS");
    RATE_LIMIT_REFILL_RATE_MS.set(rate_limit_refill_rate_ms).expect("Failed to set RATE_LIMIT_REFILL_RATE_MS");
    ADMIN_TOKEN.set(token).expect("Failed to set ADMIN_TOKEN");
    DEFAULT_CANVAS_WIDTH.set(width).expect("Failed to set DEFAULT_CANVAS_WIDTH");
    DEFAULT_CANVAS_HEIGHT.set(height).expect("Failed to set DEFAULT_CANVAS_HEIGHT");
    DEFAULT_SNAPSHOT_INTERVAL.set(snapshot_interval).expect("Failed to set DEFAULT_SNAPSHOT_INTERVAL");
    DATABASE_PATH.set(db_path).expect("Failed to set DATABASE_PATH");
    
    println!("Environment variables loaded");
    println!("Canvas size: {}x{}", width, height);
}

pub fn admin_token() -> &'static str {
    ADMIN_TOKEN.get().expect("Environment not initialized. Call env::init() first")
}

pub fn default_canvas_width() -> usize {
    *DEFAULT_CANVAS_WIDTH.get().expect("Environment not initialized. Call env::init() first")
}

pub fn default_canvas_height() -> usize {
    *DEFAULT_CANVAS_HEIGHT.get().expect("Environment not initialized. Call env::init() first")
}

pub fn default_snapshot_interval() -> usize {
    *DEFAULT_SNAPSHOT_INTERVAL.get().expect("Environment not initialized. Call env::init() first")
}

pub fn db_path() -> &'static str {
    DATABASE_PATH.get().expect("Environment not initialized. Call env::init() first")
}

pub fn rate_limit_tokens() -> f64 {
    *RATE_LIMIT_TOKENS.get().expect("Environment not initialized. Call env::init() first")
}

pub fn rate_limit_refill_rate_ms() -> f64 {
    *RATE_LIMIT_REFILL_RATE_MS.get().expect("Environment not initialized. Call env::init() first")
}