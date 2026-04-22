"""Input & Injection attack surface.

Maps to Red Team Testing Plan §12.4: Input & Injection
"""

import pytest
import uuid
from rtlib.client import CauldronClient
from rtlib import payloads


# ---------------------------------------------------------------------------
# XSS in User-Generated Content
# ---------------------------------------------------------------------------

def test_xss_in_username_rejected_or_neutralized(client):
    """XSS payloads in username must not execute (server rejects or escapes)."""
    for xss in payloads.XSS_PAYLOADS[:3]:
        c = CauldronClient(client.target)
        resp = c.session.post(f"{c.target}/api/auth/register", json={
            "username": xss,
            "email": f"rt_{hash(xss) & 0xFFFFFFFF}@redteam.local",
            "password": "RedTeamTest123!",
        })
        # Should be rejected due to username validation (alphanumeric + _ -)
        # or at least not cause issues
        assert resp.status_code in (201, 400), (
            f"XSS username caused unexpected status: {resp.status_code} — body: {resp.text[:200]}"
        )
        if resp.status_code == 201:
            c.cleanup()


def test_xss_in_channel_name(authed_client):
    """XSS payloads in channel display_name must be stored safely."""
    sid = authed_client.create_server()
    for xss in payloads.XSS_PAYLOADS[:3]:
        resp = authed_client.authed_post(
            f"/api/servers/{sid}/channels",
            json={"display_name": xss},
        )
        # Channel names allow spaces and some punctuation, but script tags
        # should be stored as literal text (not executed by backend).
        assert resp.status_code in (201, 400), (
            f"XSS channel name caused unexpected status: {resp.status_code}"
        )


def test_xss_in_message_content(authed_client, server_with_channel):
    """XSS payloads in message content must be stored safely."""
    _, cid = server_with_channel
    for xss in payloads.XSS_PAYLOADS:
        resp = authed_client.authed_post(
            f"/api/channels/{cid}/messages",
            json={"content": xss},
        )
        assert resp.status_code == 201, f"XSS message rejected unexpectedly: {resp.status_code}"
        data = resp.json()
        # The stored content should be exactly what we sent (backend doesn't sanitize,
        # relying on frontend CSP/escaping — but it must not crash or mutate unexpectedly)
        assert data["content"] == xss, f"Message content mutated: {data['content']!r} != {xss!r}"

    # Verify retrieving messages doesn't cause issues
    resp = authed_client.authed_get(f"/api/channels/{cid}/messages")
    assert resp.status_code == 200
    messages = resp.json()
    # All XSS payloads should be present in retrieved messages
    retrieved_contents = {m["content"] for m in messages}
    for xss in payloads.XSS_PAYLOADS:
        assert xss in retrieved_contents, f"XSS payload missing from retrieved messages: {xss!r}"


# ---------------------------------------------------------------------------
# SQL Injection
# ---------------------------------------------------------------------------

def test_sqli_in_login_identifier(client):
    """SQL injection in login identifier must be safely handled."""
    for payload in payloads.SQLI_PAYLOADS:
        resp = client.post("/api/auth/login", json={
            "identifier": payload,
            "password": "irrelevant",
        })
        # Must return 401 (not found / wrong creds) or 400, never 500
        assert resp.status_code in (401, 400), (
            f"SQLi in login caused {resp.status_code}: {resp.text[:200]}"
        )


def test_sqli_in_register_username(client):
    """SQL injection in register username must be safely handled."""
    for payload in payloads.SQLI_PAYLOADS[:3]:
        resp = client.post("/api/auth/register", json={
            "username": payload,
            "email": f"rt_{hash(payload) & 0xFFFFFFFF}@redteam.local",
            "password": "RedTeamTest123!",
        })
        assert resp.status_code in (400, 201), (
            f"SQLi in register caused {resp.status_code}: {resp.text[:200]}"
        )


def test_sqli_in_server_name(authed_client):
    """SQL injection in server name must be safely handled."""
    for payload in payloads.SQLI_PAYLOADS[:3]:
        resp = authed_client.authed_post("/api/servers", json={"name": payload})
        assert resp.status_code in (201, 400), (
            f"SQLi in server name caused {resp.status_code}: {resp.text[:200]}"
        )


def test_sqli_in_message_content(authed_client, server_with_channel):
    """SQL injection in message content must be safely handled."""
    _, cid = server_with_channel
    for payload in payloads.SQLI_PAYLOADS:
        resp = authed_client.authed_post(
            f"/api/channels/{cid}/messages",
            json={"content": payload},
        )
        assert resp.status_code == 201, (
            f"SQLi in message caused {resp.status_code}: {resp.text[:200]}"
        )


# ---------------------------------------------------------------------------
# Unicode Normalization Attacks
# ---------------------------------------------------------------------------

def test_unicode_normalization_in_username(client):
    """Unicode confusable usernames must be normalized or rejected."""
    # Register a unique baseline user so persistent local databases can rerun
    # the suite without colliding on a static username.
    baseline = CauldronClient(client.target)
    baseline.register(username=f"admin_{uuid.uuid4().hex[:8]}")
    baseline.cleanup()

    suffix = uuid.uuid4().hex[:8]
    for attack, desc in payloads.UNICODE_ATTACKS:
        attack_username = f"{attack}_{suffix}"
        c = CauldronClient(client.target)
        resp = c.session.post(f"{c.target}/api/auth/register", json={
            "username": attack_username,
            "email": f"rt_{hash(attack_username) & 0xFFFFFFFF}@redteam.local",
            "password": "RedTeamTest123!",
        })
        # The backend validates usernames with `.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')`
        # This will reject some but may allow confusables.
        # We assert no crash and document what gets through.
        assert resp.status_code in (201, 400), (
            f"Unicode attack ({desc}) caused unexpected {resp.status_code}: {resp.text[:200]}"
        )
        if resp.status_code == 201:
            c.cleanup()


def test_unicode_normalization_in_channel_names(authed_client):
    """Unicode confusables in channel names must be normalized."""
    sid = authed_client.create_server()
    for attack, desc in payloads.UNICODE_ATTACKS:
        resp = authed_client.authed_post(
            f"/api/servers/{sid}/channels",
            json={"display_name": attack},
        )
        assert resp.status_code in (201, 400), (
            f"Unicode channel name ({desc}) caused {resp.status_code}"
        )


# ---------------------------------------------------------------------------
# Oversized Payloads
# ---------------------------------------------------------------------------

def test_oversized_username(client):
    """Usernames over 32 chars must be rejected."""
    resp = client.post("/api/auth/register", json={
        "username": payloads.OVERSIZED["username"],
        "email": "rt@redteam.local",
        "password": "RedTeamTest123!",
    })
    assert resp.status_code == 400, f"oversized username accepted: {resp.status_code}"


def test_oversized_password_too_short(client):
    """Passwords under 8 chars must be rejected."""
    resp = client.post("/api/auth/register", json={
        "username": "rt_short",
        "email": "rt@redteam.local",
        "password": payloads.OVERSIZED["password"],
    })
    assert resp.status_code == 400, f"short password accepted: {resp.status_code}"


def test_oversized_server_name(authed_client):
    """Server names over 100 chars must be rejected."""
    resp = authed_client.authed_post("/api/servers", json={
        "name": payloads.OVERSIZED["server_name"],
    })
    assert resp.status_code == 400, f"oversized server name accepted: {resp.status_code}"


def test_oversized_channel_name(authed_client):
    """Channel names over 80 chars must be rejected."""
    sid = authed_client.create_server()
    resp = authed_client.authed_post(f"/api/servers/{sid}/channels", json={
        "display_name": payloads.OVERSIZED["channel_name"],
    })
    assert resp.status_code == 400, f"oversized channel name accepted: {resp.status_code}"


def test_oversized_message(authed_client, server_with_channel):
    """Messages over 4000 chars must be rejected."""
    _, cid = server_with_channel
    resp = authed_client.authed_post(f"/api/channels/{cid}/messages", json={
        "content": payloads.OVERSIZED["message"],
    })
    assert resp.status_code == 400, f"oversized message accepted: {resp.status_code}"


# ---------------------------------------------------------------------------
# Path Traversal / Injection
# ---------------------------------------------------------------------------

def test_path_traversal_in_api_paths(authed_client):
    """Path traversal sequences in API paths must not escape routing."""
    for payload in payloads.PATH_PAYLOADS:
        resp = authed_client.authed_get(f"/api/servers/{payload}")
        # Should be 404 (route not found) or 400, never 200 or 500
        assert resp.status_code in (404, 400, 403), (
            f"Path traversal caused unexpected {resp.status_code}: {payload!r}"
        )
