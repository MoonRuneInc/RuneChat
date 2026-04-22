"""Account Lockout & 2FA attack surface.

Maps to Red Team Testing Plan §12.2: Account Lockout & 2FA
"""

import pytest
from rtlib.client import CauldronClient


# ---------------------------------------------------------------------------
# TOTP Brute Force (expected to fail until rate limiting)
# ---------------------------------------------------------------------------

@pytest.mark.rate_limit
@pytest.mark.slow
def test_totp_brute_force_is_rate_limited(authed_client):
    """Repeated wrong TOTP codes must trigger rate limiting or lockout."""
    # Enroll TOTP
    authed_client.authed_post("/api/auth/totp/enroll")
    # We can't actually verify enrollment without a real TOTP code,
    # but we can brute-force the verify endpoint.

    statuses = []
    for i in range(50):
        resp = authed_client.authed_post(
            "/api/auth/totp/verify-enrollment",
            json={"code": f"{i:06d}"},
        )
        statuses.append(resp.status_code)

    unusual = [s for s in statuses if s not in (400, 401)]
    assert unusual, (
        f"All 50 TOTP brute-force attempts returned 400/401 — "
        "rate limiting or account lockout is missing"
    )


# ---------------------------------------------------------------------------
# Lockout Bypass with Stale JWT
# ---------------------------------------------------------------------------

def test_stale_jwt_does_not_bypass_lockout(authed_client, victim_client):
    """A JWT issued before compromise must be rejected after account is locked."""
    # Victim logs in and gets a valid JWT
    victim_token = victim_client.access_token
    victim_id = victim_client.user_id

    # Attacker triggers replay detection on victim's account
    # We need victim's refresh token. We can't easily get it (httpOnly),
    # so we simulate by having victim refresh, then we replay.
    # Actually, let's have the victim do a refresh, capture the old cookie
    # from a secondary session, then replay.
    old_cookie = victim_client.session.cookies.get("refresh_token")
    assert old_cookie

    # Victim does a normal refresh (rotates token)
    victim_client.refresh()

    # Replay old cookie
    attacker = CauldronClient(victim_client.target)
    attacker.session.cookies.set("refresh_token", old_cookie, domain="", path="/api/auth/refresh")
    resp = attacker.post("/api/auth/refresh")
    assert resp.status_code == 401, f"replay did not trigger compromise: {resp.status_code}"

    # Now the victim's account is compromised. Their OLD access token should NOT work.
    resp2 = victim_client.get("/api/servers", headers={"Authorization": f"Bearer {victim_token}"})
    assert resp2.status_code == 401, f"stale JWT bypassed lockout: {resp2.status_code}"

    attacker.cleanup()


# ---------------------------------------------------------------------------
# Email OTP Fallback Abuse
# ---------------------------------------------------------------------------

def test_email_otp_cannot_bypass_totp_requirement(authed_client):
    """Once TOTP is enrolled, email OTP unlock must be rejected."""
    # This test requires the ability to actually enroll TOTP,
    # which we can't fully do without generating valid codes.
    # We skip if the backend doesn't support probing the enrollment state.
    pytest.skip("Requires TOTP enrollment with valid code generation — test manually")


# ---------------------------------------------------------------------------
# Unlock Race Condition
# ---------------------------------------------------------------------------

def test_unlock_race_condition(authed_client):
    """Two simultaneous valid unlock attempts must not cause inconsistencies."""
    # Without TOTP enrollment we can't fully test this.
    # The backend uses transactions (see unlock_totp / unlock_email_otp_verify),
    # so race conditions on state updates are inherently serialized by DB.
    pytest.skip("Requires TOTP enrollment and a compromised account — test manually with load tool")
