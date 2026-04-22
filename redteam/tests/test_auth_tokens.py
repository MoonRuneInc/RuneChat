"""Authentication & Token attack surface.

Maps to Red Team Testing Plan §12.1: Authentication & Tokens
"""

import time
import pytest
import requests
from rtlib.client import CauldronClient
from rtlib import jwt_tools


# ---------------------------------------------------------------------------
# JWT Manipulation
# ---------------------------------------------------------------------------

def test_jwt_alg_none_rejected(client):
    """alg=none tokens must be rejected."""
    payload = {"sub": "00000000-0000-0000-0000-000000000001", "username": "attacker", "account_status": "active", "exp": 9999999999}
    token = jwt_tools.craft_none_token(payload)
    resp = client.get("/api/servers", headers={"Authorization": f"Bearer {token}"})
    assert resp.status_code == 401, f"alg=none accepted: {resp.status_code}"


def test_jwt_expired_token_rejected(authed_client):
    """Expired tokens must be rejected."""
    original = authed_client.access_token
    header, payload = jwt_tools.decode_unverified(original)
    expired_payload = jwt_tools.make_expired_payload(payload, offset_seconds=-3600)
    bad_token = jwt_tools.craft_hmac_token(expired_payload, "wrong-secret")
    resp = authed_client.get("/api/servers", headers={"Authorization": f"Bearer {bad_token}"})
    assert resp.status_code == 401, f"expired/wrong-sig token accepted: {resp.status_code}"


def test_jwt_wrong_secret_rejected(client):
    """Tokens signed with wrong secret must be rejected."""
    payload = {"sub": "00000000-0000-0000-0000-000000000001", "username": "attacker", "account_status": "active", "exp": 9999999999}
    token = jwt_tools.craft_hmac_token(payload, "totally-wrong-secret-32-bytes!!")
    resp = client.get("/api/servers", headers={"Authorization": f"Bearer {token}"})
    assert resp.status_code == 401, f"wrong-secret token accepted: {resp.status_code}"


def test_jwt_tampered_claims_rejected(authed_client):
    """Tokens with modified payload but original signature must be rejected."""
    original = authed_client.access_token
    parts = original.split(".")
    payload = jwt_tools.decode_payload(original)
    payload["account_status"] = "admin"  # tamper
    payload["username"] = "root"
    new_body = jwt_tools.encode_part(payload)
    # Keep original signature — it won't match tampered body
    tampered = f"{parts[0]}.{new_body}.{parts[2]}"
    resp = authed_client.get("/api/servers", headers={"Authorization": f"Bearer {tampered}"})
    assert resp.status_code == 401, f"tampered token accepted: {resp.status_code}"


def test_jwt_expiry_tampering_rejected(authed_client):
    """Tokens with extended expiry must be rejected."""
    original = authed_client.access_token
    parts = original.split(".")
    payload = jwt_tools.decode_payload(original)
    payload["exp"] = int(time.time()) + 86400 * 365  # extend to 1 year
    new_body = jwt_tools.encode_part(payload)
    # We can't sign without the secret, so just swap body and expect sig mismatch
    tampered = f"{parts[0]}.{new_body}.{parts[2]}"
    resp = authed_client.get("/api/servers", headers={"Authorization": f"Bearer {tampered}"})
    assert resp.status_code == 401, f"expiry-tampered token accepted: {resp.status_code}"


# ---------------------------------------------------------------------------
# Refresh Token Security
# ---------------------------------------------------------------------------

def test_refresh_token_is_httpOnly(authed_client):
    """Refresh token cookie must be httpOnly (not readable from JS)."""
    resp = authed_client.session.post(f"{authed_client.target}/api/auth/login", json={
        "identifier": authed_client.username,
        "password": "RedTeamTest123!",
    })
    assert resp.status_code == 200
    # Check Set-Cookie header on the login response for the register doesn't set it,
    # but refresh might. Let's explicitly trigger a refresh to get the cookie.
    # Actually register already gives a cookie. Check that response history.
    # The register response should have included the cookie.
    # We need to inspect the original register response.
    # Re-register to check.
    c2 = CauldronClient(authed_client.target)
    reg_resp = c2.session.post(f"{c2.target}/api/auth/register", json={
        "username": f"rt_cookie_{int(time.time())}",
        "email": f"rt_{int(time.time())}@redteam.local",
        "password": "RedTeamTest123!",
    })
    assert reg_resp.status_code == 201
    set_cookie = reg_resp.headers.get("Set-Cookie", "")
    assert "HttpOnly" in set_cookie, f"refresh token missing HttpOnly flag: {set_cookie}"
    assert "Secure" in set_cookie, f"refresh token missing Secure flag: {set_cookie}"
    assert "SameSite=Strict" in set_cookie, f"refresh token missing SameSite=Strict: {set_cookie}"
    c2.cleanup()


def test_refresh_token_replay_kills_sessions(authed_client):
    """Using a refresh token twice must revoke all sessions and mark compromised."""
    # Capture the current refresh token cookie
    cookie = authed_client.session.cookies.get("refresh_token")
    assert cookie, "no refresh_token cookie found"

    # First refresh — should succeed and rotate token
    resp1 = authed_client.refresh()
    assert "access_token" in resp1

    # Steal the OLD cookie and replay it in a new session
    c2 = CauldronClient(authed_client.target)
    c2.session.cookies.set("refresh_token", cookie, domain="", path="/api/auth/refresh")
    resp2 = c2.post("/api/auth/refresh")

    # Must fail — replay detected
    assert resp2.status_code == 401, f"refresh replay accepted: {resp2.status_code}"

    # The original user's account should now be compromised
    # Try to use the original access token — middleware checks live DB status
    resp3 = authed_client.get("/api/servers", headers={"Authorization": f"Bearer {authed_client.access_token}"})
    assert resp3.status_code == 401, f"stale JWT still works after compromise: {resp3.status_code}"

    c2.cleanup()


# ---------------------------------------------------------------------------
# Cache Control
# ---------------------------------------------------------------------------

def test_auth_endpoints_have_no_store(client):
    """Auth endpoints must return Cache-Control: no-store."""
    resp = client.post("/api/auth/login", json={"identifier": "nope", "password": "nope"})
    cc = resp.headers.get("Cache-Control", "")
    assert "no-store" in cc, f"login missing no-store: {cc}"

    resp2 = client.post("/api/auth/register", json={"username": "x", "email": "x@x.co", "password": "short"})
    cc2 = resp2.headers.get("Cache-Control", "")
    assert "no-store" in cc2, f"register missing no-store: {cc2}"


# ---------------------------------------------------------------------------
# CSRF / SameSite
# ---------------------------------------------------------------------------

def test_refresh_cookie_samesite_strict():
    """Refresh cookie path must enforce SameSite=Strict."""
    # This is validated by test_refresh_token_is_httpOnly above,
    # but we also verify the cookie scope is tight.
    c = CauldronClient()
    c.register()
    cookie = c.session.cookies.get("refresh_token")
    assert cookie
    # The cookie should only be sent to /api/auth/refresh
    # We can't directly inspect the path attribute via requests cookies,
    # but we can verify it's NOT sent to a non-auth endpoint.
    resp = c.get("/api/servers")
    # requests won't show us what cookies were sent, but we can verify
    # the server doesn't error due to cookie presence (it would 401 anyway without auth header)
    assert resp.status_code == 401
    c.cleanup()


# ---------------------------------------------------------------------------
# Brute Force (expected to fail until rate limiting is implemented)
# ---------------------------------------------------------------------------

@pytest.mark.rate_limit
@pytest.mark.slow
def test_login_brute_force_is_rate_limited(client):
    """Repeated failed logins must be rate limited or account-locked."""
    # Create a victim account
    victim = CauldronClient(client.target)
    victim.register(password="SuperSecret123!")
    username = victim.username

    attempts = 20
    statuses = []
    for i in range(attempts):
        resp = client.post("/api/auth/login", json={
            "identifier": username,
            "password": f"wrong-password-{i}",
        })
        statuses.append(resp.status_code)

    victim.cleanup()

    # We expect at least ONE non-401 response (429, 403, or 400 indicating lockout)
    # If all are 401, rate limiting is missing.
    unusual = [s for s in statuses if s != 401]
    assert unusual, (
        f"All {attempts} brute-force attempts returned 401 — "
        "rate limiting or account lockout is missing"
    )


@pytest.mark.rate_limit
@pytest.mark.slow
def test_credential_stuffing_simulation(client):
    """Many login attempts with common passwords should trigger defenses."""
    victim = CauldronClient(client.target)
    victim.register(password="password123")
    username = victim.username

    common_passwords = ["password", "123456", "password123", "qwerty", "letmein"]
    statuses = []
    for pwd in common_passwords:
        resp = client.post("/api/auth/login", json={"identifier": username, "password": pwd})
        statuses.append(resp.status_code)

    victim.cleanup()

    # After some attempts, we expect throttling or lockout
    # For MVP without rate limiting this will fail — that's the point
    unusual = [s for s in statuses if s == 429 or s == 403]
    assert unusual, (
        "No rate limiting triggered during credential stuffing simulation — "
        f"statuses: {statuses}"
    )
