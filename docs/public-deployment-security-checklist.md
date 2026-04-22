# Cauldron Public Deployment Security Checklist

> **Purpose:** Answer "Can we safely expose this to the public internet today?"
>
> **Scope:** MVP (accounts, servers, channels, invites, real-time messaging).
>
> **Derived from:** Red Team suite results, architecture spec §12, production compose
> review, CVE review, and release checklist requirements.

---

## 1. Blocking Gates

Public launch stops if any item is unchecked. These are must-pass.

### 1.1 Code Verification

- [ ] **Backend unit tests pass** (run against a CI or local test DB — **never production**)
  ```bash
  DATABASE_URL=postgres://<ci-or-local-test-db> cargo test -p cauldron-backend
  ```
  Expected: 22+ unit tests passed, 0 failed. Auth regression test passed.

- [ ] **Red Team suite passes**
  Pre-deploy internal validation (local Compose):
  ```bash
  cd redteam && source .venv/bin/activate && pytest -v --tb=short
  ```
  Production launch verification (live target):
  ```bash
  cd redteam && source .venv/bin/activate && \
    CAULDRON_TARGET=https://chat.moonrune.cc pytest -v --tb=short
  ```
  Expected: 49 passed, 8 skipped, 0 failed.
  The 4 rate-limit tests (`test_login_brute_force_is_rate_limited`,
  `test_credential_stuffing_simulation`, `test_totp_brute_force_is_rate_limited`,
  `test_invite_enumeration_is_rate_limited`) must all pass.

- [ ] **No uncommitted changes on `master`**
  ```bash
  git diff --stat HEAD
  ```
  Expected: empty.

### 1.2 Infrastructure Hardening

- [ ] **Managed PostgreSQL in use** — self-hosted `postgres:16-alpine` with known
  container-layer CVEs is **not** acceptable for public launch. See
  `docs/superpowers/plans/2026-04-19-plan-cve-review.md`.
  - [ ] `DATABASE_URL` points to managed instance (Neon, Supabase, Railway, RDS, etc.)
  - [ ] `docker-compose.prod.yml` does **not** contain a `db` service
  - [ ] Postgres port 5432 is **not** published on the host

- [ ] **Redis is internal only**
  - [ ] `docker-compose.prod.yml` has no `ports` on the `redis` service
  - [ ] Port scan from outside confirms 6379 is closed:
    ```bash
    nmap -p 6379 <host>
    ```

- [ ] **Proxy binds safely**
  - [ ] `docker-compose.prod.yml` proxy publishes `127.0.0.1:8080:80` (not `80:80`)
    unless the host is directly behind an upstream LB that handles TLS
  - [ ] `nginx/prod.conf` uses `real_ip` module so `$remote_addr` reflects the
    actual client IP for rate limiting

- [ ] **TLS termination configured**
  - [ ] Upstream load balancer (Cloudflare, AWS ALB, nginx, etc.) terminates TLS
  - [ ] Plain HTTP to port 80 either redirects to HTTPS or is rejected
  - [ ] `DOMAIN` env var matches the TLS certificate name

### 1.3 Secrets & Configuration

- [ ] **`.env.prod` exists and is complete**
  ```bash
  make prod-config
  ```
  Expected: config validates without missing-variable errors.
  Required vars present:
  - `DATABASE_URL` (managed Postgres)
  - `JWT_SECRET` (≥32 bytes, unique per deployment)
  - `TOTP_ENCRYPTION_KEY` (32 bytes base64, unique per deployment)
  - `DOMAIN` (matches public hostname)

- [ ] **No dev secrets in production**
  - [ ] `JWT_SECRET` is not the dev default or example value
  - [ ] `TOTP_ENCRYPTION_KEY` is not the dev default or example value
  - [ ] `.env.prod` is **not** committed to git

- [ ] **Error responses do not leak secrets**
  ```bash
  curl -s http://localhost:8080/api/auth/login -X POST -d "not json" \
    -H "Content-Type: application/json" | grep -iE "jwt_secret|database_url|password|smtp_password|totp_encryption_key"
  ```
  Expected: no matches.

### 1.4 Release Readiness

- [ ] **Release workflow reviewed**
  - [ ] `.github/workflows/release.yml` and `.gitea/workflows/release.yml` are the
    versions that passed static review
  - [ ] A release watcher with Gitea/GitHub UI access is assigned and available
  - [ ] `docs/release-checklist.md` is understood by the watcher

---

## 2. Operational Requirements

These are not code gates — they are things the operator must have in place.

| Requirement | Why | Minimum Standard |
|---|---|---|
| Managed PostgreSQL | CVE posture + backups + HA | Neon, Supabase, Railway, or RDS |
| `.env.prod` | Secrets management | File exists, gitignored, permissions 600 |
| Release watcher | First release is unverified end-to-end | Person with UI access, checklist in hand |
| TLS / Load Balancer | Transport security | Terminates TLS, forwards to `127.0.0.1:8080` |
| Backups | Data recovery | Managed DB auto-backups + tested restore |
| Monitoring basics | Incident response | Health endpoint probed (`/health` → 200), logs shipped |
| Domain + DNS | Reachability | `chat.moonrune.cc` → LB IP |

---

## 3. Security Verification

Run these commands on the production host (or equivalent staging environment) before
launch. Record output in the vault.

### 3.1 Red Team Suite

```bash
cd redteam && source .venv/bin/activate && pytest -v --tb=short
```
**Expected:** 49 passed, 8 skipped, 0 failed.

### 3.2 Backend Tests

```bash
DATABASE_URL=postgres://<ci-or-local-test-db> cargo test -p cauldron-backend
```
Do not use `.env.prod` or the production managed Postgres URL for this command.
**Expected:** 22+ passed, 0 failed.

### 3.3 Compose Config Validation

```bash
make prod-config
```
**Expected:** Valid YAML, no missing-variable errors, no `db` service,
proxy publishes `127.0.0.1:8080:80`.

### 3.4 Nginx Config Syntax

```bash
make prod-nginx-test
```
**Expected:** `syntax is ok`, `test is successful`.

### 3.5 Port Exposure Scan

```bash
# From a host outside the Docker network
nmap -p 3000,5432,6379,8080 <production-host>
```
**Expected:** Only the LB port (e.g., 443) is open. 3000, 5432, 6379, 8080
are filtered/closed from the public internet. (8080 may be open to the LB
if the LB runs on a different host.)

### 3.6 CVE Posture Check

```bash
# If self-hosted Postgres is still in the picture anywhere
docker scout cves postgres:16-alpine
```
**Expected:** Not applicable — managed Postgres should be in use.
If a local dev container is scanned, document the findings and confirm
it is NOT the production database.

### 3.7 Health Endpoint

```bash
curl -s https://chat.moonrune.cc/health
```
**Expected:** `{"status":"ok"}` with response time < 1s.

---

## 4. Known Risk Register

These are accepted risks for MVP. They must have an owner and a review trigger.

| Risk | Owner | Acceptance | Review Trigger |
|---|---|---|---|
| Release workflow unverified end-to-end | Rhea Solis | First real tag is treated as release candidate, watched live | After first successful release |
| No CSP (Content Security Policy) | Maya Kade | XSS payloads are stored but not executed; frontend escaping is the current defense | Before adding user-generated HTML or markdown rendering |
| Rate limits are in-memory only | Maya Kade | Governor keyed buckets are per-process; single-instance deploy is assumed | Before horizontal scaling or multi-instance deploy |
| No audit log | Iris Vale | Login events, token usage, and privilege changes are not surfaced | Federation or compliance requirements |
| No E2EE | Iris Vale | Messages are encrypted in transit (TLS) but not at rest or end-to-end | High-trust community requirement |
| WebSocket connection flooding not rate limited | Maya Kade | WS accepts connections with valid JWT; no per-IP or per-connection flood limit | Load testing shows degradation under flood |
| No automated DB migration rollback | Maya Kade | Migrations are forward-only; rollback requires manual intervention | After first production incident requiring rollback |
| Redis is single-instance, no persistence configured | Maya Kade | Redis pub/sub is ephemeral; WS broker state is lost on restart | Before adding presence/online status features |

---

## 5. Launch Decision

> **Do not sign off unless all Blocking Gates (§1) are checked.**

| Field | Value |
|---|---|
| **Decision** | ☐ GO / ☐ NO-GO |
| **Date** | |
| **Signer** | |
| **Role** | |
| **Commit/tag** | |
| **Environment** | `chat.moonrune.cc` |

### Residual Risks at Launch

List any risks from §4 that are still open and why they are accepted:

1.
2.
3.

### Notes

_Record anything unusual observed during verification, or decisions made during sign-off._
