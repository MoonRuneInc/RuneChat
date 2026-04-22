"""Infrastructure attack surface.

Maps to Red Team Testing Plan §12.5: Infrastructure
"""

import os
import socket
import pytest
from rtlib.client import CauldronClient, DEFAULT_TARGET


# ---------------------------------------------------------------------------
# Secret Leakage
# ---------------------------------------------------------------------------

def test_error_responses_do_not_leak_secrets(client):
    """Internal errors must return generic messages without secrets."""
    # Trigger a 500 by sending malformed JSON
    resp = client.post("/api/auth/login", data="not json", headers={"Content-Type": "application/json"})
    body = resp.text.lower()
    # Must not contain sensitive keywords
    forbidden = ["jwt_secret", "database_url", "redis_url", "password", "smtp_password", "totp_encryption_key"]
    for keyword in forbidden:
        assert keyword not in body, f"Error response leaked potential secret: {keyword}"


def test_500_error_is_generic(client):
    """Server errors must return generic 'internal server error'."""
    resp = client.post("/api/auth/login", data="not json", headers={"Content-Type": "application/json"})
    if resp.status_code == 500:
        body = resp.json()
        assert body.get("error") == "internal server error", (
            f"500 error leaked details: {body}"
        )


def test_headers_do_not_expose_server_info(client):
    """HTTP headers must not expose framework/version details."""
    resp = client.get("/health")
    server_header = resp.headers.get("Server", "")
    x_powered = resp.headers.get("X-Powered-By", "")
    # If Server header exists, it should not contain Rust/Axum specifics
    assert "axum" not in server_header.lower(), f"Server header exposes framework: {server_header}"
    assert "rust" not in x_powered.lower(), f"X-Powered-By exposes stack: {x_powered}"


# ---------------------------------------------------------------------------
# Network Exposure (deployment-dependent)
# ---------------------------------------------------------------------------

@pytest.mark.deployment
def test_redis_not_externally_accessible():
    """Redis port 6379 must not be open on the host."""
    target = os.environ.get("CAULDRON_TARGET", DEFAULT_TARGET)
    host = target.replace("http://", "").replace("https://", "").split(":")[0]
    if host in ("localhost", "127.0.0.1"):
        pytest.skip("Localhost testing — Redis exposure check is deployment-specific")

    try:
        sock = socket.create_connection((host, 6379), timeout=2)
        sock.close()
        pytest.fail("Redis port 6379 is externally accessible")
    except (socket.timeout, ConnectionRefusedError, OSError):
        pass  # Expected — port is closed


@pytest.mark.deployment
def test_postgres_not_externally_accessible():
    """PostgreSQL port 5432 must not be open on the host."""
    target = os.environ.get("CAULDRON_TARGET", DEFAULT_TARGET)
    host = target.replace("http://", "").replace("https://", "").split(":")[0]
    if host in ("localhost", "127.0.0.1"):
        pytest.skip("Localhost testing — PostgreSQL exposure check is deployment-specific")

    try:
        sock = socket.create_connection((host, 5432), timeout=2)
        sock.close()
        pytest.fail("PostgreSQL port 5432 is externally accessible")
    except (socket.timeout, ConnectionRefusedError, OSError):
        pass  # Expected


@pytest.mark.deployment
def test_tls_enforced():
    """Production deployment must redirect HTTP to HTTPS or reject plain HTTP."""
    target = os.environ.get("CAULDRON_TARGET", DEFAULT_TARGET)
    if target.startswith("https://"):
        # Already HTTPS — try HTTP version
        http_target = target.replace("https://", "http://")
        try:
            import requests
            resp = requests.get(http_target, timeout=5, allow_redirects=False)
            # Should redirect (301/308) or be rejected (400/403)
            assert resp.status_code in (301, 308, 400, 403), (
                f"HTTP access allowed with status {resp.status_code} — TLS not enforced"
            )
        except requests.exceptions.ConnectionError:
            pass  # Port 80 closed — also acceptable
    elif target.startswith("http://localhost"):
        pytest.skip("Localhost — TLS enforcement is deployment-specific")
    else:
        pytest.skip("Cannot determine TLS enforcement for this target")
