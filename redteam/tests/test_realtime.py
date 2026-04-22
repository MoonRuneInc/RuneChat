"""Real-time attack surface.

Maps to Red Team Testing Plan §12.6: Real-time
"""

import asyncio
import pytest
from rtlib.client import CauldronClient


def test_ws_without_token_rejected(client):
    """WebSocket connection without a token must be rejected."""
    import websocket
    ws_url = client.target.replace("http://", "ws://").replace("https://", "wss://")
    try:
        ws = websocket.create_connection(f"{ws_url}/ws")
        ws.close()
        pytest.fail("WebSocket accepted connection without token")
    except Exception:
        pass  # Expected — connection rejected


def test_ws_with_invalid_token_rejected(client):
    """WebSocket connection with invalid token must be rejected."""
    import websocket
    ws_url = client.target.replace("http://", "ws://").replace("https://", "wss://")
    try:
        ws = websocket.create_connection(f"{ws_url}/ws?token=invalid.jwt.here")
        ws.close()
        pytest.fail("WebSocket accepted connection with invalid token")
    except Exception:
        pass  # Expected


def test_ws_origin_validation_blocks_foreign_origin(authed_client):
    """WebSocket from foreign Origin must be rejected (CSWSH protection)."""
    import websocket
    ws_url = authed_client.target.replace("http://", "ws://").replace("https://", "wss://")
    try:
        ws = websocket.create_connection(
            f"{ws_url}/ws?token={authed_client.access_token}",
            origin="https://evil.com",
        )
        ws.close()
        pytest.fail("WebSocket accepted foreign origin")
    except Exception:
        pass  # Expected — origin check blocks it


def test_ws_valid_origin_allowed(authed_client):
    """WebSocket from valid Origin must succeed."""
    import websocket
    ws_url = authed_client.target.replace("http://", "ws://").replace("https://", "wss://")
    try:
        ws = websocket.create_connection(
            f"{ws_url}/ws?token={authed_client.access_token}",
            origin="http://localhost:5173",
            timeout=3,
        )
        ws.close()
        # Success — no exception
    except Exception as e:
        pytest.fail(f"Valid origin WebSocket failed: {e}")


def test_ws_compromised_account_rejected(authed_client):
    """WebSocket connection for a compromised account must be rejected."""
    # Trigger compromise via refresh token replay
    old_cookie = authed_client.session.cookies.get("refresh_token")
    assert old_cookie
    authed_client.refresh()  # rotate

    attacker = CauldronClient(authed_client.target)
    attacker.session.cookies.set("refresh_token", old_cookie, domain="", path="/api/auth/refresh")
    attacker.post("/api/auth/refresh")  # replay → compromise

    # Now try WS with original token
    import websocket
    ws_url = authed_client.target.replace("http://", "ws://").replace("https://", "wss://")
    try:
        ws = websocket.create_connection(
            f"{ws_url}/ws?token={authed_client.access_token}",
            origin="http://localhost:5173",
        )
        ws.close()
        pytest.fail("WebSocket accepted compromised account token")
    except Exception:
        pass  # Expected — compromised account blocked

    attacker.cleanup()


@pytest.mark.rate_limit
@pytest.mark.slow
def test_ws_connection_flooding():
    """Rapid WS connections must be rate limited."""
    pytest.skip("Rate limiting not implemented — test with load tool after rate limits added")
