use super::*;
use std::env;

#[test]
fn defaults_without_env() {
    unsafe {
        env::remove_var("PORT");
        env::remove_var("ENVIRONMENT");
        env::remove_var("API_KEYS");
        env::remove_var("MAX_UPLOAD_MB");
        env::remove_var("PALETTES_DIR");
    }
    let cfg = Config::from_env();
    assert_eq!(cfg.port, 3000);
    assert_eq!(cfg.environment, Environment::Development);
    assert!(cfg.api_keys.is_none());
    assert_eq!(cfg.max_upload_bytes, 50 * 1024 * 1024);
    assert!(!cfg.is_production());
    assert!(!cfg.auth_enabled());
}

#[test]
fn production_environment_flag() {
    unsafe { env::set_var("ENVIRONMENT", "production"); }
    let cfg = Config::from_env();
    assert!(cfg.is_production());
    unsafe { env::remove_var("ENVIRONMENT"); }
}

#[test]
fn api_keys_parsed_from_csv() {
    unsafe { env::set_var("API_KEYS", "key_a, key_b, key_c"); }
    let cfg = Config::from_env();
    let keys = cfg.api_keys.unwrap();
    assert!(keys.contains("key_a"));
    assert!(keys.contains("key_b"));
    assert!(keys.contains("key_c"));
    unsafe { env::remove_var("API_KEYS"); }
}
