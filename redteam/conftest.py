"""pytest fixtures for red team tests."""

import os
import pytest
from rtlib.client import CauldronClient, DEFAULT_TARGET


def pytest_report_header(config):
    target = os.environ.get("CAULDRON_TARGET", DEFAULT_TARGET)
    return f"cauldron-redteam: target={target}"


@pytest.fixture
def client():
    """Unauthenticated client."""
    c = CauldronClient()
    yield c
    c.cleanup()


@pytest.fixture
def authed_client():
    """Client logged in as a fresh user."""
    c = CauldronClient()
    c.register()
    yield c
    c.cleanup()


@pytest.fixture
def victim_client():
    """Second authenticated client for cross-user tests."""
    c = CauldronClient()
    c.register()
    yield c
    c.cleanup()


@pytest.fixture
def server_with_channel(authed_client):
    """Returns (server_id, channel_id) for the authed client's server."""
    sid = authed_client.create_server()
    cid = authed_client.create_channel(sid)
    return sid, cid
