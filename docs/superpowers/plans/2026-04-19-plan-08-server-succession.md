# Plan 08 — Server Succession & Deadman Protocol

> **Author:** Iris Vale  
> **Date:** 2026-04-19  
> **Status:** Planned — post-MVP, pre-public-launch  
> **Goal:** Ensure no community permanently loses its server due to an owner account incident, and give users a human-verifiable way to confirm identity recovery out of band.

---

## Problem

A server owner's account can become inaccessible through:
- Account compromise (replay attack detected → status = `compromised`)
- Platform suspension (status = `suspended`)
- Voluntary departure without transferring ownership

Under the current schema, if an owner is locked out, the server is frozen — no one can manage membership, promote admins, or handle moderation. The community is held hostage to the owner's account state.

---

## Design

### Trigger conditions

A succession event is opened automatically when:
- The server owner's `account_status` becomes `compromised` or `suspended`
- (Future: owner manually hands off control before going offline long-term)

### What the owner loses during an active succession event

The owner's account is **locked from server management actions only**:
- Cannot delete the server
- Cannot change member roles
- Cannot modify server settings
- Cannot revoke or generate invites

They retain full membership: they can still read and send messages. Succession is about protecting the community, not punishing the owner.

### Succession configuration (set by owner before any incident)

Stored in a new `server_succession` table:

| Field | Purpose |
|---|---|
| `designated_successor_id` | The admin the owner explicitly trusts to act if they're locked out. Falls back to oldest admin by `joined_at` if not set. |
| `backup_account_id` | A second account the owner personally controls. Ownership can be transferred here during recovery. |
| `recovery_hint` | A short question (not an answer) the owner sets in the app. Displayed to the designated successor when they're initiating a transfer. **The answer is never entered into RuneChat — it is communicated verbally.** |

### Recovery paths

**Path 1 — Owner recovers their account**
- Owner completes TOTP or email OTP unlock
- Succession event closes automatically
- Owner regains full server management rights

**Path 2 — Transfer to backup account**
1. Designated successor sees the succession event notification
2. Successor initiates ownership transfer to the pre-registered backup account
3. App shows the successor the `recovery_hint`
4. Successor contacts the backup account holder out of band (phone call, in person) and asks the question
5. If satisfied, successor confirms in the app
6. Backup account holder is prompted to verify identity (TOTP or email OTP)
7. Ownership transfers; original owner is demoted to admin

**Path 3 — No backup account configured**
- Successor is prompted to become permanent owner, with an explicit confirmation step
- Original owner is notified (via email if configured) that they have a window to contest before the transfer finalises

### The verbal recovery hint — user guidance

The `recovery_hint` is a question only. **RuneChat never stores or verifies the answer.**

Example hints a user might set:
- "Ask me what I said when we met"
- "What's the name of the first server we ran together?"
- "What did I tell you not to tell anyone?"

The goal is to give a trusted person a quick way to confirm "the person claiming to be my friend actually is my friend" — not to add another cryptographic gate. This is explicitly a **social trust layer** on top of the technical one.

During server setup and in server settings, the app will prompt:
> "Set a recovery hint — a question only you and a trusted person know the answer to. Tell them the answer in person or over a phone call. Never send it digitally. This helps your designated successor confirm your identity if you ever need to recover your server."

---

## Schema additions

```sql
CREATE TABLE server_succession (
    server_id                   UUID PRIMARY KEY REFERENCES servers(id) ON DELETE CASCADE,
    designated_successor_id     UUID REFERENCES users(id) ON DELETE SET NULL,
    backup_account_id           UUID REFERENCES users(id) ON DELETE SET NULL,
    recovery_hint               TEXT,
    configured_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE server_succession_events (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id           UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    trigger             TEXT NOT NULL CHECK (trigger IN ('compromised', 'suspended', 'manual')),
    triggered_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    acting_caretaker_id UUID REFERENCES users(id) ON DELETE SET NULL,
    resolved_at         TIMESTAMPTZ,
    resolution          TEXT CHECK (resolution IN ('owner_recovered', 'transferred', 'cancelled'))
);

CREATE INDEX idx_succession_events_server ON server_succession_events (server_id, resolved_at);
```

The `servers` table and `server_members` table require no changes. Succession is a layer on top.

---

## API surface (new endpoints)

| Endpoint | Who | Purpose |
|---|---|---|
| `PUT /api/servers/:id/succession` | Owner | Configure successor, backup account, recovery hint |
| `GET /api/servers/:id/succession` | Owner, designated successor | View current configuration |
| `GET /api/servers/:id/succession/event` | Designated successor | Check if a succession event is active |
| `POST /api/servers/:id/succession/event/transfer` | Designated successor | Initiate transfer to backup account |
| `POST /api/servers/:id/succession/event/accept` | Backup account | Accept ownership (triggers TOTP/email verification) |
| `POST /api/servers/:id/succession/event/cancel` | Owner (on recovery) | Cancel active event after regaining account |

---

## UI surface

**Server settings — Succession tab**
- Designate successor (dropdown of current admins)
- Register backup account (enter username of the account you control)
- Set recovery hint
- Warning banner: "Tell your designated successor the answer to your recovery hint in person or over a voice call — never in a message"

**Designated successor view (when event is active)**
- Banner: "[Owner] is locked out. You are acting caretaker."
- Button to initiate ownership transfer
- Recovery hint displayed with reminder: "Verify the answer with [backup account holder] out of band before proceeding"

**Server list / server header**
- Subtle indicator when a succession event is active on a server you administer

---

## Implementation tasks for Maya

- [ ] Migration: `server_succession` and `server_succession_events` tables
- [ ] Auth middleware: intercept owner-privileged actions, return 423 Locked when succession event is active for that server
- [ ] Succession API: all 6 endpoints
- [ ] Trigger hook: when `account_status` changes to `compromised`/`suspended`, open succession events for any owned servers
- [ ] Frontend: Succession settings tab in server settings
- [ ] Frontend: Active succession event banners and transfer flow
- [ ] Notification: alert designated successor when event opens (in-app + email if configured)

---

## Open questions (resolve before implementation)

1. **Transfer window for Path 3 (no backup):** How long does the original owner have to contest before the successor becomes permanent owner? Proposed: 14 days with email notification.
2. **Multiple servers:** If an owner runs several servers and their account is compromised, do all succession events open simultaneously? Answer: yes — a compromised account should not retain management rights over any server.
3. **Successor is also compromised:** What happens if the designated successor's account is also locked? Fallback: next admin by `joined_at`. Needs a depth limit (e.g., 3 levels).

---

## What this is NOT

- Not a platform-level account deletion / right-to-be-forgotten mechanism (separate feature)
- Not a forced moderation tool for the platform to seize servers
- Not a cryptographic proof system — the verbal hint is intentionally human-scale

---

## Placement in roadmap

Post-MVP, pre-public-launch. The feature is important for the user-first promise but does not block team testing or invite-only beta. It should ship before RuneChat opens to the general public.
