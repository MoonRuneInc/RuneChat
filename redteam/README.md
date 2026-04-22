# Cauldron Red Team Test Suite

Executable security tests covering all attack surfaces defined in the architecture spec's Red Team Testing Plan. Run these before any public deployment.

## Quick Start

```bash
cd redteam
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt

# Against local Docker Compose (default)
pytest -v

# Against a deployed instance
CAULDRON_TARGET=https://chat.moonrune.cc pytest -v

# Generate HTML report
pytest -v --html=report.html --self-contained-html
```

## Prerequisites

- Python 3.11+
- A running Cauldron backend (local Docker Compose or deployed)
- The backend must allow registration (set `DISABLE_REGISTRATION=false` if enforced)

## Test Categories

| File | Coverage |
|---|---|
| `test_auth_tokens.py` | JWT attacks, refresh token replay, cookie security, CSRF |
| `test_2fa_lockout.py` | TOTP brute force, lockout bypass, email OTP abuse |
| `test_authorization.py` | IDOR, privilege escalation, invite races |
| `test_input_injection.py` | XSS, SQLi, Unicode attacks, oversized payloads |
| `test_infrastructure.py` | Secret leakage, TLS, network exposure |
| `test_realtime.py` | WebSocket hijacking, unauthorized subscription, flooding |

## Interpreting Results

- **PASS** — Defense is working as intended.
- **FAIL** — A vulnerability or missing defense was found. Fix before deploying.
- **SKIP** — Test requires a condition not met (e.g., external network access).
- **XFAIL** — Known gap documented in the architecture. Expected to fail until fixed.

## Design Notes

- Tests are self-contained: each creates its own users/servers/channels and cleans up.
- Rate-limiting tests will **fail** if no rate limiting is configured. This is intentional — they document the gap.
- Infrastructure tests (TLS, Redis/PostgreSQL exposure) are deployment-specific and may SKIP in local environments.
