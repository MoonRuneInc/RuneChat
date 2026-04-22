# Changelog

## v0.1.3 - 2026-04-22

Android signature verification recovery.

### Fixed

- Resolve `apksigner` from the installed Android build-tools directory instead of relying on runner `PATH`.

## v0.1.2 - 2026-04-22

Android release workflow recovery.

### Fixed

- Make generated Android Gradle signing imports idempotent.
- Write Android keystore properties without heredoc indentation risk.
- Resolve the Android upload keystore from the generated Gradle root project.

## v0.1.1 - 2026-04-22

Android signing recovery release.

### Fixed

- Removed the unsigned Android APK failure path from the release workflow.
- Require Android signing secrets before publishing Android release artifacts.
- Verify Android APK signatures before upload.
- Exclude `*-unsigned.apk` files from release artifacts.

### Notes

- `v0.1.0` remains Windows-only after the invalid unsigned Android APK was removed from the GitHub release.

## v0.1.0 - 2026-04-22

First watched Cauldron release candidate.

### Added

- Security-first MVP chat platform with account registration, login, refresh-token rotation, logout, and compromised-account unlock flows.
- Server, channel, invite, message history, and WebSocket real-time messaging support.
- TOTP enrollment and encrypted 2FA secret storage.
- Rate limiting for login, TOTP verification, invite preview, and invite join surfaces.
- Red Team test suite covering authentication, authorization, input handling, infrastructure, and real-time security behavior.
- Production Docker Compose, nginx, and environment templates for managed PostgreSQL deployments.
- TrueNAS SCALE deployment artifacts and Cloudflare Tunnel configuration templates.
- GitHub and Gitea tag-based release workflows for Windows and Android client artifacts.

### Security

- Refresh token replay detection invalidates active sessions after compromise.
- Invite join race protection and authorization checks are verified by tests.
- Production proxy configuration preserves client IP and forwarded protocol headers for rate limiting and Cloudflare Tunnel deployments.
- Public deployment checklist added with blocking gates, operational requirements, verification commands, and known risk register.

### Notes

- Public deployment still requires an operator-managed PostgreSQL decision and live release workflow observation.
- The first `v0.1.0` tag is intentionally treated as a watched release candidate.
