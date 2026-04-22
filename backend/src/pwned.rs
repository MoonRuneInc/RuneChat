use sha1::{Digest, Sha1};

#[derive(Debug, Clone, PartialEq)]
pub enum PwnedCheckResult {
    Clean,
    Pwned { count: u64 },
    ServiceUnavailable,
}

/// Check a password against the Have I Been Pwned Pwned Passwords API.
/// Uses k-anonymity: only the first 5 hex chars of the SHA-1 hash are sent.
pub async fn check_password(client: &reqwest::Client, password: &str) -> PwnedCheckResult {
    check_password_with_base_url(client, password, "https://api.pwnedpasswords.com").await
}

/// Internal variant that accepts a base URL for testing.
pub(crate) async fn check_password_with_base_url(
    client: &reqwest::Client,
    password: &str,
    base_url: &str,
) -> PwnedCheckResult {
    let hash = format!("{:X}", Sha1::digest(password.as_bytes()));
    let prefix = &hash[..5];
    let suffix = &hash[5..];

    let url = format!("{}/range/{}", base_url, prefix);

    let response = match client
        .get(&url)
        .header("Add-Padding", "true")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => r,
        _ => return PwnedCheckResult::ServiceUnavailable,
    };

    let body = match response.text().await {
        Ok(b) => b,
        Err(_) => return PwnedCheckResult::ServiceUnavailable,
    };

    parse_pwned_response(&body, suffix)
}

/// Parse a HIBP range response body and check whether the given suffix appears.
pub(crate) fn parse_pwned_response(body: &str, suffix: &str) -> PwnedCheckResult {
    for line in body.lines() {
        if let Some((line_suffix, count_str)) = line.split_once(':') {
            if line_suffix.eq_ignore_ascii_case(suffix) {
                let count = count_str.trim().parse().unwrap_or(1);
                return PwnedCheckResult::Pwned { count };
            }
        }
    }

    PwnedCheckResult::Clean
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn check_password_finds_pwned_password_via_mock() {
        let mock_server = MockServer::start().await;
        let hash = format!("{:X}", Sha1::digest(b"password"));
        let prefix = &hash[..5];
        let suffix = &hash[5..];

        let response_body = format!("00001:1\n{}:42\nFFFFF:0\n", suffix);

        Mock::given(matchers::method("GET"))
            .and(matchers::path(format!("/range/{}", prefix)))
            .and(matchers::header("Add-Padding", "true"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let result = check_password_with_base_url(&client, "password", &mock_server.uri()).await;

        assert_eq!(result, PwnedCheckResult::Pwned { count: 42 });
    }

    #[tokio::test]
    async fn check_password_returns_clean_for_unknown_password() {
        let mock_server = MockServer::start().await;
        let hash = format!(
            "{:X}",
            Sha1::digest(b"this-is-a-unique-cauldron-test-password-42")
        );
        let prefix = &hash[..5];

        // Response does not contain our suffix
        let response_body = "00001:1\nABCDE:99\nFFFFF:0\n";

        Mock::given(matchers::method("GET"))
            .and(matchers::path(format!("/range/{}", prefix)))
            .and(matchers::header("Add-Padding", "true"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let result = check_password_with_base_url(
            &client,
            "this-is-a-unique-cauldron-test-password-42",
            &mock_server.uri(),
        )
        .await;

        assert_eq!(result, PwnedCheckResult::Clean);
    }

    #[tokio::test]
    async fn check_password_returns_service_unavailable_on_500() {
        let mock_server = MockServer::start().await;
        let hash = format!("{:X}", Sha1::digest(b"password"));
        let prefix = &hash[..5];

        Mock::given(matchers::method("GET"))
            .and(matchers::path(format!("/range/{}", prefix)))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let result = check_password_with_base_url(&client, "password", &mock_server.uri()).await;

        assert_eq!(result, PwnedCheckResult::ServiceUnavailable);
    }

    #[tokio::test]
    async fn check_password_returns_service_unavailable_on_timeout() {
        let mock_server = MockServer::start().await;
        let hash = format!("{:X}", Sha1::digest(b"password"));
        let prefix = &hash[..5];

        // Respond after 10 seconds — longer than our 5-second timeout
        Mock::given(matchers::method("GET"))
            .and(matchers::path(format!("/range/{}", prefix)))
            .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(10)))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let result = check_password_with_base_url(&client, "password", &mock_server.uri()).await;

        assert_eq!(result, PwnedCheckResult::ServiceUnavailable);
    }

    #[test]
    fn parse_pwned_response_finds_match() {
        let suffix = "ABCDE";
        let body = format!("00001:1\n{}:42\nFFFFF:0\n", suffix);

        let result = parse_pwned_response(&body, suffix);
        assert_eq!(result, PwnedCheckResult::Pwned { count: 42 });
    }

    #[test]
    fn parse_pwned_response_no_match() {
        let suffix = "ZZZZZ";
        let body = "00001:1\nABCDE:42\nFFFFF:0\n";

        let result = parse_pwned_response(&body, suffix);
        assert_eq!(result, PwnedCheckResult::Clean);
    }

    #[test]
    fn parse_pwned_response_case_insensitive_match() {
        let body = "00001:1\nabcde:7\nFFFFF:0\n";

        let result = parse_pwned_response(&body, "ABCDE");
        assert_eq!(result, PwnedCheckResult::Pwned { count: 7 });
    }

    #[test]
    fn parse_pwned_response_malformed_count_defaults_to_1() {
        let body = "00001:1\nABCDE:bad\nFFFFF:0\n";

        let result = parse_pwned_response(&body, "ABCDE");
        assert_eq!(result, PwnedCheckResult::Pwned { count: 1 });
    }

    #[test]
    fn sha1_hash_format_is_uppercase_hex() {
        let hash = format!("{:X}", Sha1::digest(b"password"));
        assert_eq!(hash.len(), 40);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(hash
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_numeric()));
    }

    #[test]
    fn prefix_is_first_5_chars() {
        let hash = format!("{:X}", Sha1::digest(b"test"));
        let prefix = &hash[..5];
        assert_eq!(prefix.len(), 5);
    }
}
