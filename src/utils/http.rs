use std::sync::LazyLock;
use std::time::Duration;

pub static DEFAULT_HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(concat!("xavier-internal/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("failed to build default HTTP client")
});
