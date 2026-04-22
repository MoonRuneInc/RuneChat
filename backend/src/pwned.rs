use sha1::{Digest, Sha1};

#[derive(Debug, Clone, PartialEq)]
pub enum PwnedCheckResult {
    Clean,
    Pwned { count: u64 },
    ServiceUnavailable,
}

pub async fn check_password(client: &reqwest::Client, password: &str) -> PwnedCheckResult {
    let hash = format!("{:X}", Sha1::digest(password.as_bytes()));
    let prefix = &hash[..5];
    let suffix = &hash[5..];

    let url = format!("https://api.pwnedpasswords.com/range/{}", prefix);

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

    #[test]
    fn parse_pwned_response_finds_match() {
        let suffix = "ABCDE";
        let body = format!(
            "00001:1\n{}:42\nFFFFF:0\n",
            suffix
        );

        let found = body.lines().any(|line| {
            if let Some((line_suffix, count_str)) = line.split_once(':') {
                line_suffix.eq_ignore_ascii_case(suffix)
                    && count_str.trim().parse::<u64>().unwrap_or(0) > 0
            } else {
                false
            }
        });

        assert!(found);
    }

    #[test]
    fn parse_pwned_response_no_match() {
        let suffix = "ZZZZZ";
        let body = "00001:1\nABCDE:42\nFFFFF:0\n";

        let found = body.lines().any(|line| {
            if let Some((line_suffix, _)) = line.split_once(':') {
                line_suffix.eq_ignore_ascii_case(suffix)
            } else {
                false
            }
        });

        assert!(!found);
    }

    #[test]
    fn sha1_hash_format_is_uppercase_hex() {
        let hash = format!("{:X}", Sha1::digest(b"password"));
        assert_eq!(hash.len(), 40);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(hash.chars().all(|c| c.is_ascii_uppercase() || c.is_numeric()));
    }

    #[test]
    fn prefix_is_first_5_chars() {
        let hash = format!("{:X}", Sha1::digest(b"test"));
        let prefix = &hash[..5];
        assert_eq!(prefix.len(), 5);
    }
}
