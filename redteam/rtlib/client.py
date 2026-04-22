"""HTTP client wrapper with auth helpers and cleanup tracking."""

import os
import uuid
from typing import Optional
import requests

DEFAULT_TARGET = os.environ.get("CAULDRON_TARGET", "http://localhost:8080")


class CauldronClient:
    """Session-based HTTP client for Cauldron API testing."""

    def __init__(self, target: str = DEFAULT_TARGET):
        self.target = target.rstrip("/")
        self.session = requests.Session()
        self.access_token: Optional[str] = None
        self.user_id: Optional[str] = None
        self.username: Optional[str] = None
        # Track resources for cleanup
        self._servers: list[str] = []

    # --- Low-level ---

    def get(self, path: str, **kwargs):
        return self.session.get(f"{self.target}{path}", **kwargs)

    def post(self, path: str, **kwargs):
        if (
            path == "/api/auth/refresh"
            and self.target.startswith("http://")
            and "headers" not in kwargs
        ):
            cookie = self.session.cookies.get("refresh_token")
            if cookie:
                kwargs["headers"] = {"Cookie": f"refresh_token={cookie}"}
        return self.session.post(f"{self.target}{path}", **kwargs)

    def delete(self, path: str, **kwargs):
        return self.session.delete(f"{self.target}{path}", **kwargs)

    def authed_get(self, path: str, **kwargs):
        h = kwargs.pop("headers", {})
        h["Authorization"] = f"Bearer {self.access_token}"
        return self.get(path, headers=h, **kwargs)

    def authed_post(self, path: str, **kwargs):
        h = kwargs.pop("headers", {})
        h["Authorization"] = f"Bearer {self.access_token}"
        return self.post(path, headers=h, **kwargs)

    def authed_delete(self, path: str, **kwargs):
        h = kwargs.pop("headers", {})
        h["Authorization"] = f"Bearer {self.access_token}"
        return self.delete(path, headers=h, **kwargs)

    # --- Auth helpers ---

    def register(self, username: Optional[str] = None, password: str = "RedTeamTest123!") -> dict:
        """Register a new user and return the auth response."""
        username = username or f"rt_{uuid.uuid4().hex[:8]}"
        email = f"{username}@redteam.local"
        resp = self.post("/api/auth/register", json={
            "username": username,
            "email": email,
            "password": password,
        })
        resp.raise_for_status()
        data = resp.json()
        self.access_token = data["access_token"]
        self.user_id = data["user"]["id"]
        self.username = data["user"]["username"]
        return data

    def login(self, identifier: str, password: str) -> dict:
        """Login and store tokens."""
        resp = self.post("/api/auth/login", json={
            "identifier": identifier,
            "password": password,
        })
        resp.raise_for_status()
        data = resp.json()
        self.access_token = data["access_token"]
        self.user_id = data["user"]["id"]
        self.username = data["user"]["username"]
        return data

    def logout(self):
        """Logout and clear local token."""
        if self.access_token:
            try:
                self.authed_post("/api/auth/logout")
            except Exception:
                pass
        self.access_token = None
        self.user_id = None
        self.username = None

    def refresh(self) -> dict:
        """Hit the refresh endpoint (requires refresh_token cookie)."""
        resp = self.post("/api/auth/refresh")
        resp.raise_for_status()
        data = resp.json()
        self.access_token = data["access_token"]
        return data

    # --- Resource helpers ---

    def create_server(self, name: Optional[str] = None) -> str:
        """Create a server, return its ID, track for cleanup."""
        name = name or f"rt-server-{uuid.uuid4().hex[:6]}"
        resp = self.authed_post("/api/servers", json={"name": name})
        resp.raise_for_status()
        sid = resp.json()["id"]
        self._servers.append(sid)
        return sid

    def create_channel(self, server_id: str, name: str = "general") -> str:
        """Create a channel, return its ID."""
        resp = self.authed_post(f"/api/servers/{server_id}/channels", json={"display_name": name})
        resp.raise_for_status()
        return resp.json()["id"]

    def send_message(self, channel_id: str, content: str) -> dict:
        resp = self.authed_post(f"/api/channels/{channel_id}/messages", json={"content": content})
        resp.raise_for_status()
        return resp.json()

    def cleanup(self):
        """Delete tracked servers and logout."""
        if self.access_token:
            for sid in self._servers:
                try:
                    self.authed_delete(f"/api/servers/{sid}")
                except Exception:
                    pass
            self.logout()
        self.session.close()

    def __enter__(self):
        return self

    def __exit__(self, *args):
        self.cleanup()
