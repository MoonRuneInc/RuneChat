# Plan 13: Member List + Default Channels Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a persistent member list panel to the chat UI and automatically seed every new server with General and Announcements channels.

**Architecture:** Two independent changes — (1) a backend-only addition to the `create_server` transaction that inserts two default channels, and (2) a new frontend `MemberList` component wired into `ChatPage` as a fourth column, consuming the existing `GET /api/servers/:id/members` endpoint. No schema changes or new API endpoints required.

**Tech Stack:** Rust/axum/sqlx (backend), React/TypeScript/TanStack Query/Tailwind (frontend)

---

## File Map

| File | Change |
|---|---|
| `backend/src/api/servers.rs` | Add two channel inserts to `create_server` transaction |
| `backend/tests/server_defaults.rs` | New integration test: default channels on server creation |
| `frontend/src/api/client.ts` | Add `Member` interface and `membersApi.list()` |
| `frontend/src/components/MemberList.tsx` | New component: grouped-by-role member panel |
| `frontend/src/pages/ChatPage.tsx` | Add `MemberList` as fourth column |

---

## Task 1: Default channels on server creation (backend)

**Files:**
- Modify: `backend/src/api/servers.rs` (the `create_server` function, around line 44–82)
- Create: `backend/tests/server_defaults.rs`

### Step 1: Write the failing test

- [ ] Create `backend/tests/server_defaults.rs`:

```rust
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrations = "./migrations")]
async fn create_server_seeds_general_and_announcements(pool: PgPool) {
    // Insert a user to act as the server owner
    let user_id: Uuid = sqlx::query_scalar(
        "INSERT INTO users (username, email, password_hash) VALUES ('dani', 'dani@example.com', 'hash') RETURNING id"
    )
    .fetch_one(&pool)
    .await
    .expect("insert user");

    // Insert a server directly (bypassing the HTTP handler) and its owner membership
    let server_id: Uuid = sqlx::query_scalar(
        "INSERT INTO servers (name, owner_id) VALUES ('Test Server', $1) RETURNING id"
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("insert server");

    sqlx::query(
        "INSERT INTO server_members (server_id, user_id, role) VALUES ($1, $2, 'owner')"
    )
    .bind(server_id)
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("insert owner membership");

    // This test will FAIL until Task 1 Step 3 is implemented — no channels exist yet
    let channels: Vec<(String, String)> = sqlx::query_as(
        "SELECT display_name, slug FROM channels WHERE server_id = $1 ORDER BY created_at ASC"
    )
    .bind(server_id)
    .fetch_all(&pool)
    .await
    .expect("list channels");

    assert_eq!(channels.len(), 2, "new server must have exactly 2 default channels");
    assert_eq!(channels[0], ("General".to_string(), "general".to_string()));
    assert_eq!(channels[1], ("Announcements".to_string(), "announcements".to_string()));
}
```

### Step 2: Run the test — confirm it fails

- [ ] Run:
```bash
DATABASE_URL=postgres://cauldron:cauldron@localhost:5432/cauldron cargo test -p cauldron-backend --test server_defaults 2>&1 | tail -20
```
Expected: `FAILED` — `assertion failed: channels.len() == 2` (channels are empty).

### Step 3: Add default channel inserts to `create_server`

- [ ] In `backend/src/api/servers.rs`, replace the `create_server` function body. The full updated function (lines 44–82 currently):

```rust
async fn create_server(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateServerBody>,
) -> crate::error::Result<(StatusCode, Json<ServerResponse>)> {
    let name = body.name.trim().to_string();
    if name.is_empty() || name.len() > 100 {
        return Err(AppError::BadRequest(
            "server name must be 1-100 characters".to_string(),
        ));
    }

    let mut tx = state.db.begin().await?;

    let server_id: Uuid =
        sqlx::query_scalar("INSERT INTO servers (name, owner_id) VALUES ($1, $2) RETURNING id")
            .bind(&name)
            .bind(auth.user_id)
            .fetch_one(&mut *tx)
            .await?;

    // Add creator as owner in server_members
    sqlx::query("INSERT INTO server_members (server_id, user_id, role) VALUES ($1, $2, 'owner')")
        .bind(server_id)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await?;

    // Seed default channels
    for (display_name, slug) in [("General", "general"), ("Announcements", "announcements")] {
        sqlx::query(
            "INSERT INTO channels (server_id, display_name, slug) VALUES ($1, $2, $3)",
        )
        .bind(server_id)
        .bind(display_name)
        .bind(slug)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(ServerResponse {
            id: server_id,
            name,
            owner_id: auth.user_id,
            member_count: 1,
            my_role: "owner".to_string(),
        }),
    ))
}
```

### Step 4: Run the test — confirm it passes

- [ ] Run:
```bash
DATABASE_URL=postgres://cauldron:cauldron@localhost:5432/cauldron cargo test -p cauldron-backend --test server_defaults 2>&1 | tail -10
```
Expected: `test create_server_seeds_general_and_announcements ... ok`

### Step 5: Run the full test suite — confirm no regressions

- [ ] Run:
```bash
DATABASE_URL=postgres://cauldron:cauldron@localhost:5432/cauldron cargo test -p cauldron-backend 2>&1 | tail -10
```
Expected: all tests pass, 0 failed.

### Step 6: Commit

- [ ] Run:
```bash
git add backend/src/api/servers.rs backend/tests/server_defaults.rs
git commit -m "feat(backend): seed General and Announcements channels on server creation"
```

---

## Task 2: Member API type + client method

**Files:**
- Modify: `frontend/src/api/client.ts` (after the `invitesApi` block, around line 150)

### Step 1: Add `Member` interface and `membersApi`

- [ ] In `frontend/src/api/client.ts`, append after the `invitesApi` block:

```typescript
// --- Members ---
export interface Member {
  user_id: string
  username: string
  role: string
  joined_at: string
}

export const membersApi = {
  list: (serverId: string) => request<Member[]>(`/servers/${serverId}/members`),
}
```

### Step 2: Verify TypeScript compiles

- [ ] Run:
```bash
cd frontend && npx tsc --noEmit
```
Expected: no errors.

### Step 3: Commit

- [ ] Run:
```bash
git add frontend/src/api/client.ts
git commit -m "feat(frontend): add Member type and membersApi.list"
```

---

## Task 3: MemberList component

**Files:**
- Create: `frontend/src/components/MemberList.tsx`

### Step 1: Create the component

- [ ] Create `frontend/src/components/MemberList.tsx`:

```tsx
import { useQuery } from '@tanstack/react-query'
import { membersApi, type Member } from '../api/client'

// Role display config. When server-specific role customization is added,
// fetch this from the API and pass it as a prop with these values as fallback.
const ROLE_CONFIG: Record<string, { label: string; color: string }> = {
  owner: { label: 'Owner', color: '#f5a623' },
  admin: { label: 'Admin', color: '#63c5ff' },
  member: { label: '', color: '' },
}

const ROLE_ORDER = ['owner', 'admin', 'member']

interface Props {
  serverId: string
}

function avatarColor(username: string): string {
  const colors = ['#6c63ff', '#63c5ff', '#ff6b9d', '#51cf66', '#f5a623', '#ff6348']
  let hash = 0
  for (const ch of username) hash = (hash * 31 + ch.charCodeAt(0)) & 0xffffffff
  return colors[Math.abs(hash) % colors.length]
}

export default function MemberList({ serverId }: Props) {
  const { data: members = [] } = useQuery({
    queryKey: ['members', serverId],
    queryFn: () => membersApi.list(serverId),
    staleTime: 30_000,
  })

  const grouped = ROLE_ORDER.reduce<Record<string, Member[]>>((acc, role) => {
    acc[role] = members.filter((m) => m.role === role)
    return acc
  }, {})

  return (
    <div className="w-48 flex flex-col bg-surface-800 border-l border-surface-700 shrink-0">
      <div className="px-3 py-3 border-b border-surface-700">
        <span className="text-xs font-semibold uppercase text-ivory/50 tracking-wide">
          Members — {members.length}
        </span>
      </div>

      <div className="flex-1 overflow-y-auto scrollbar-thin py-2">
        {ROLE_ORDER.map((role) => {
          const group = grouped[role]
          if (group.length === 0) return null
          const config = ROLE_CONFIG[role] ?? { label: role, color: '' }
          const sectionLabel = config.label
            ? `${config.label.toUpperCase()}S — ${group.length}`
            : `MEMBERS — ${group.length}`

          return (
            <div key={role} className="mb-3">
              <div className="px-3 mb-1">
                <span className="text-xs font-semibold uppercase text-ivory/40 tracking-wide">
                  {sectionLabel}
                </span>
              </div>
              {group.map((member) => {
                const cfg = ROLE_CONFIG[member.role] ?? { label: '', color: '' }
                return (
                  <div
                    key={member.user_id}
                    className="flex items-center gap-2 px-3 py-1.5 hover:bg-surface-700/40 rounded mx-1"
                  >
                    <div
                      className="w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold text-white shrink-0"
                      style={{ background: avatarColor(member.username) }}
                    >
                      {member.username[0]?.toUpperCase()}
                    </div>
                    <div className="min-w-0">
                      <div className="text-xs text-ivory truncate">{member.username}</div>
                      {cfg.label && (
                        <div className="text-xs truncate" style={{ color: cfg.color }}>
                          {cfg.label}
                        </div>
                      )}
                    </div>
                  </div>
                )
              })}
            </div>
          )
        })}
      </div>
    </div>
  )
}
```

### Step 2: Verify TypeScript compiles

- [ ] Run:
```bash
cd frontend && npx tsc --noEmit
```
Expected: no errors.

### Step 3: Verify lint passes

- [ ] Run:
```bash
cd frontend && npm run lint
```
Expected: no errors.

### Step 4: Commit

- [ ] Run:
```bash
git add frontend/src/components/MemberList.tsx
git commit -m "feat(frontend): add MemberList component grouped by role"
```

---

## Task 4: Wire MemberList into ChatPage

**Files:**
- Modify: `frontend/src/pages/ChatPage.tsx`

### Step 1: Import and add MemberList to the layout

- [ ] In `frontend/src/pages/ChatPage.tsx`, add the import after the existing component imports:

```tsx
import MemberList from '../components/MemberList'
```

- [ ] In the main return (the four-column layout, after the closing `</div>` of the message area div and before the modals), add:

```tsx
{serverId && <MemberList serverId={serverId} />}
```

The full updated return block (replace the existing `return (` block starting at line 76):

```tsx
  return (
    <div className="h-screen flex bg-surface-900 overflow-hidden">
      <ServerList onCreateServer={() => setShowCreateServer(true)} />

      {serverId && currentServer && (
        <ChannelList
          serverId={serverId}
          serverName={currentServer.name}
          onCreateChannel={() => setShowCreateChannel(true)}
        />
      )}

      {channelId && currentChannel ? (
        <div className="flex-1 flex flex-col min-w-0">
          {/* Channel header */}
          <div className="h-12 border-b border-surface-700 flex items-center px-4 gap-2 shrink-0">
            <span className="text-ivory/50">#</span>
            <span className="font-semibold text-ivory">{currentChannel.display_name}</span>
            <div className="flex-1" />
            {serverId && (
              <button
                onClick={() => setShowInvite(true)}
                className="text-xs text-ivory/60 hover:text-ivory px-2 py-1 rounded hover:bg-surface-700"
              >
                Invite
              </button>
            )}
          </div>

          {/* Compromised banner */}
          {isCompromised && (
            <div className="px-4 pt-3">
              <CompromisedBanner username={user!.username} />
            </div>
          )}

          <MessageList messages={messages} isLoading={isLoading} />

          <MessageInput
            channelId={channelId}
            channelName={currentChannel.display_name}
            disabled={isCompromised}
            disabledReason="Your account is locked. Unlock it to send messages."
          />
        </div>
      ) : (
        <div className="flex-1 flex items-center justify-center text-ivory/50 text-sm">
          {channels.length === 0 ? 'No channels yet — create one!' : 'Select a channel'}
        </div>
      )}

      {serverId && <MemberList serverId={serverId} />}

      {showCreateServer && <ServerCreateModal onClose={() => setShowCreateServer(false)} />}
      {showCreateChannel && serverId && (
        <ChannelCreateModal serverId={serverId} onClose={() => setShowCreateChannel(false)} />
      )}
      {showInvite && serverId && (
        <InviteModal serverId={serverId} onClose={() => setShowInvite(false)} />
      )}
    </div>
  )
```

### Step 2: Verify TypeScript compiles

- [ ] Run:
```bash
cd frontend && npx tsc --noEmit
```
Expected: no errors.

### Step 3: Verify lint passes

- [ ] Run:
```bash
cd frontend && npm run lint
```
Expected: no errors.

### Step 4: Build to confirm no runtime import errors

- [ ] Run:
```bash
cd frontend && npm run build 2>&1 | tail -10
```
Expected: build completes, no errors.

### Step 5: Smoke test in browser

- [ ] Start the stack: `docker compose up -d`
- [ ] Start Vite dev server: `cd frontend && npm run dev`
- [ ] Open `http://localhost:5173` in a browser
- [ ] Log in and select a server
- [ ] Verify: a member panel appears on the right side, showing the current user grouped under OWNERS
- [ ] Create a new server via the UI
- [ ] Verify: the new server immediately has **General** and **Announcements** channels in the channel list
- [ ] Verify: the member list shows 1 member (yourself) under OWNERS

### Step 6: Commit

- [ ] Run:
```bash
git add frontend/src/pages/ChatPage.tsx
git commit -m "feat(frontend): add persistent member list panel to chat layout"
```

---

## Future: Role Customization Hook

When server-specific role names and colors are added (e.g. `GET /api/servers/:id/roles`), the change is:

1. Add a `roles` field to the `Server` interface or fetch a separate endpoint
2. Pass the fetched config into `MemberList` as a `roleConfig?: Record<string, { label: string; color: string }>` prop
3. Inside `MemberList`, replace `ROLE_CONFIG` with `props.roleConfig ?? ROLE_CONFIG`

No structural changes to the component are needed — the `ROLE_CONFIG` constant is the only hook point.
