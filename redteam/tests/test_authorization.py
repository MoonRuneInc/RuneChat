"""Authorization & Access Control attack surface.

Maps to Red Team Testing Plan §12.3: Authorization & Access Control
"""

import uuid
import concurrent.futures
import pytest
from rtlib.client import CauldronClient


# ---------------------------------------------------------------------------
# IDOR: Insecure Direct Object Reference
# ---------------------------------------------------------------------------

def test_idor_server_listing(victim_client, authed_client):
    """User A must not see User B's servers in list."""
    # Victim creates a private server
    victim_sid = victim_client.create_server(name="Secret Base")

    # Attacker lists servers
    resp = authed_client.authed_get("/api/servers")
    assert resp.status_code == 200
    servers = resp.json()
    sids = [s["id"] for s in servers]
    assert victim_sid not in sids, "IDOR: attacker can see victim's servers"


def test_idor_server_details(victim_client, authed_client):
    """User A must not fetch details of User B's server."""
    victim_sid = victim_client.create_server()
    resp = authed_client.authed_get(f"/api/servers/{victim_sid}")
    assert resp.status_code in (403, 404), f"IDOR on server details: {resp.status_code}"


def test_idor_channel_listing(victim_client, authed_client):
    """User A must not list channels in User B's server."""
    victim_sid = victim_client.create_server()
    victim_client.create_channel(victim_sid, "secrets")
    resp = authed_client.authed_get(f"/api/servers/{victim_sid}/channels")
    assert resp.status_code in (403, 404), f"IDOR on channel listing: {resp.status_code}"


def test_idor_channel_details(victim_client, authed_client):
    """User A must not fetch channel details from User B's server."""
    victim_sid = victim_client.create_server()
    cid = victim_client.create_channel(victim_sid, "secrets")
    resp = authed_client.authed_get(f"/api/channels/{cid}")
    assert resp.status_code in (403, 404), f"IDOR on channel details: {resp.status_code}"


def test_idor_messages(victim_client, authed_client):
    """User A must not read messages from User B's channel."""
    victim_sid = victim_client.create_server()
    cid = victim_client.create_channel(victim_sid, "general")
    victim_client.send_message(cid, "secret message")
    resp = authed_client.authed_get(f"/api/channels/{cid}/messages")
    assert resp.status_code in (403, 404), f"IDOR on messages: {resp.status_code}"


def test_idor_send_message_to_foreign_channel(victim_client, authed_client):
    """User A must not send messages to User B's channel."""
    victim_sid = victim_client.create_server()
    cid = victim_client.create_channel(victim_sid, "general")
    resp = authed_client.authed_post(f"/api/channels/{cid}/messages", json={"content": "hacked"})
    assert resp.status_code in (403, 404), f"IDOR on send message: {resp.status_code}"


# ---------------------------------------------------------------------------
# Privilege Escalation
# ---------------------------------------------------------------------------

def test_member_cannot_delete_server(authed_client, victim_client):
    """A regular member must not delete a server."""
    # Victim creates server
    sid = victim_client.create_server()
    # Invite authed_client to join
    # We need an invite code. Create one.
    inv_resp = victim_client.authed_post("/api/invite", json={"server_id": sid})
    assert inv_resp.status_code == 201
    code = inv_resp.json()["code"]

    # Join as member
    join_resp = authed_client.authed_post(f"/api/invite/{code}/join")
    assert join_resp.status_code == 200

    # Try to delete server as member
    resp = authed_client.authed_delete(f"/api/servers/{sid}")
    assert resp.status_code == 403, f"member deleted server: {resp.status_code}"


def test_member_cannot_kick_owner(authed_client, victim_client):
    """A member must not kick the server owner."""
    sid = victim_client.create_server()
    inv_resp = victim_client.authed_post("/api/invite", json={"server_id": sid})
    code = inv_resp.json()["code"]
    authed_client.authed_post(f"/api/invite/{code}/join")

    resp = authed_client.authed_delete(f"/api/servers/{sid}/members/{victim_client.user_id}")
    assert resp.status_code == 403, f"member kicked owner: {resp.status_code}"


def test_admin_cannot_kick_admin(authed_client, victim_client):
    """An admin must not kick another admin (only owner can)."""
    # This requires role management which is not in MVP (no promote-to-admin endpoint).
    pytest.skip("Admin promotion endpoint not in MVP — test manually")


def test_non_member_cannot_delete_channel(authed_client, victim_client):
    """A user not in the server must not delete a channel."""
    sid = victim_client.create_server()
    cid = victim_client.create_channel(sid)
    resp = authed_client.authed_delete(f"/api/channels/{cid}")
    assert resp.status_code in (403, 404), f"non-member deleted channel: {resp.status_code}"


# ---------------------------------------------------------------------------
# Invite Security
# ---------------------------------------------------------------------------

@pytest.mark.rate_limit
@pytest.mark.slow
def test_invite_enumeration_is_rate_limited(client):
    """Brute-forcing invite codes must be rate limited."""
    codes = ["AAAAAA", "BBBBBB", "CCCCCC", "12345678", "ABCDEFGH"]
    statuses = []
    for code in codes:
        resp = client.get(f"/api/invite/{code}")
        statuses.append(resp.status_code)

    # After a few attempts, should be throttled
    unusual = [s for s in statuses if s == 429]
    assert unusual, (
        f"No rate limiting on invite enumeration — statuses: {statuses}"
    )


def test_invite_race_condition_max_uses(authed_client, victim_client):
    """Two simultaneous joins to a max_uses=1 invite must result in only one success."""
    sid = victim_client.create_server()
    inv_resp = victim_client.authed_post("/api/invite", json={
        "server_id": sid,
        "max_uses": 1,
    })
    assert inv_resp.status_code == 201
    code = inv_resp.json()["code"]

    # Create two fresh users who will race to join
    u1 = CauldronClient(authed_client.target)
    u1.register()
    u2 = CauldronClient(authed_client.target)
    u2.register()

    results = []

    def join(client):
        try:
            r = client.authed_post(f"/api/invite/{code}/join")
            return r.status_code
        except Exception as e:
            return str(e)

    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as ex:
        futures = [ex.submit(join, u1), ex.submit(join, u2)]
        for f in concurrent.futures.as_completed(futures):
            results.append(f.result())

    u1.cleanup()
    u2.cleanup()

    successes = [r for r in results if r == 200]
    assert len(successes) == 1, (
        f"Race condition: {len(successes)} successes out of 2 joins — results: {results}"
    )


def test_join_via_expired_invite(authed_client, victim_client):
    """Joining via an expired invite must fail."""
    sid = victim_client.create_server()
    inv_resp = victim_client.authed_post("/api/invite", json={
        "server_id": sid,
        "expires_in_hours": -1,  # already expired
    })
    assert inv_resp.status_code == 201
    code = inv_resp.json()["code"]

    resp = authed_client.authed_post(f"/api/invite/{code}/join")
    assert resp.status_code in (400, 404), f"expired invite accepted: {resp.status_code}"


def test_join_via_exhausted_invite(authed_client, victim_client):
    """Joining via an exhausted invite must fail."""
    sid = victim_client.create_server()
    inv_resp = victim_client.authed_post("/api/invite", json={
        "server_id": sid,
        "max_uses": 1,
    })
    code = inv_resp.json()["code"]

    # First join succeeds
    r1 = authed_client.authed_post(f"/api/invite/{code}/join")
    assert r1.status_code == 200

    # Second join fails
    u2 = CauldronClient(authed_client.target)
    u2.register()
    r2 = u2.authed_post(f"/api/invite/{code}/join")
    u2.cleanup()
    assert r2.status_code in (400, 404, 409), f"exhausted invite accepted: {r2.status_code}"


# ---------------------------------------------------------------------------
# WebSocket Authorization
# ---------------------------------------------------------------------------

def test_ws_subscription_without_membership(authed_client, victim_client):
    """Connecting WS to a foreign server's channel must be rejected or not receive data."""
    # The MVP WebSocket handler doesn't do channel-level subscription yet;
    # it just connects the user and pushes all messages for all their channels.
    # So this test validates that the WS connection itself requires valid auth.
    pytest.skip("Channel-level WS subscription not yet implemented — verify at Plan 6 UAT")
