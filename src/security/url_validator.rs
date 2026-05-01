use reqwest::Url;
use std::net::IpAddr;

/// Validates a URL intended for internal Xavier2 communication.
/// Prevents SSRF by blocking access to sensitive metadata services and
/// ensuring the URL matches an optional allowlist.
pub fn validate_internal_url(url_str: &str) -> Result<Url, String> {
    let url = Url::parse(url_str).map_err(|e| format!("Invalid URL format: {}", e))?;

    // 1. Block non-HTTP/HTTPS schemes
    match url.scheme() {
        "http" | "https" => {}
        _ => return Err(format!("Unsupported scheme: {}", url.scheme())),
    }

    // 2. Validate Host
    let host = url.host_str().ok_or("URL must have a host")?;

    // 3. Block known metadata services
    let forbidden_hosts = [
        "169.254.169.254",
        "metadata.google.internal",
        "metadata",
        "instance-data",
        "100.100.100.200", // Alibaba Cloud
    ];

    if forbidden_hosts
        .iter()
        .any(|&h| host.eq_ignore_ascii_case(h))
    {
        return Err(format!("Forbidden host detected: {}", host));
    }

    // 4. IP-based validation for private/link-local addresses
    if let Ok(ip) = host.parse::<IpAddr>() {
        if ip.is_loopback() {
            // Allow loopback for local development if not explicitly forbidden
            // but we might want to toggle this via env.
        } else if let IpAddr::V4(v4) = ip {
            // Check for link-local (169.254.x.x)
            if v4.is_link_local() {
                return Err("Link-local addresses are forbidden".to_string());
            }
        }
        // Also check if it's a multicast address
        if ip.is_multicast() {
            return Err("Multicast addresses are forbidden".to_string());
        }
    }

    // 5. Allowlist validation (optional)
    if let Ok(allowlist) = std::env::var("XAVIER2_ALLOWED_DOMAINS") {
        let domains: Vec<&str> = allowlist.split(',').collect();
        if !domains.iter().any(|&d| host.eq_ignore_ascii_case(d.trim())) {
            return Err(format!(
                "Host '{}' is not in the allowed domains list",
                host
            ));
        }
    }

    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_urls() {
        assert!(validate_internal_url("http://localhost:8006").is_ok());
        assert!(validate_internal_url("https://example.com").is_ok());
        assert!(validate_internal_url("http://127.0.0.1:8006").is_ok());
    }

    #[test]
    fn test_validate_invalid_format() {
        assert!(validate_internal_url("not-a-url").is_err());
    }

    #[test]
    fn test_validate_unsupported_scheme() {
        assert!(validate_internal_url("ftp://localhost").is_err());
        assert!(validate_internal_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn test_validate_forbidden_hosts() {
        assert!(validate_internal_url("http://169.254.169.254").is_err());
        assert!(validate_internal_url("http://metadata.google.internal").is_err());
        assert!(validate_internal_url("http://METADATA").is_err());
    }

    #[test]
    fn test_validate_link_local() {
        assert!(validate_internal_url("http://169.254.1.1").is_err());
    }

    #[test]
    fn test_validate_allowlist() {
        std::env::set_var("XAVIER2_ALLOWED_DOMAINS", "local.host, internal.corp");

        assert!(validate_internal_url("http://local.host").is_ok());
        assert!(validate_internal_url("http://internal.corp").is_ok());
        assert!(validate_internal_url("http://example.com").is_err());

        std::env::remove_var("XAVIER2_ALLOWED_DOMAINS");
    }
}
