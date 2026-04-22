# Cauldron Release Checklist

> For the release watcher assigned before `v0.1.0` and every release thereafter.
> Do not treat the first release tag as a blind ship — it is a release candidate that
> must be watched live.

---

## Pre-Release

- [ ] `master` is green (backend tests pass, Red Team suite passes).
- [ ] `AGENTS.md`, `CHANGELOG`, and version strings are updated.
- [ ] `.github/workflows/release.yml` and `.gitea/workflows/release.yml` have not changed
  since the last successful static review.
- [ ] A release watcher with Gitea/GitHub UI access is assigned and available during
  the release window.

---

## Live Release Steps

### 1. Cut the tag

```bash
git tag -a v0.1.0 -m "Cauldron v0.1.0"
git push origin v0.1.0
git push github v0.1.0   # if GitHub mirror is manually synced
```

### 2. Watch the workflow start

- [ ] Tag push triggered the workflow within 60 seconds.
- [ ] Both `build-windows` and `build-android` jobs are queued or running.

### 3. Verify Windows job

- [ ] Job status changes from `queued` → `in_progress` → `passed`.
- [ ] NSIS installation step completed (no infinite retry loops).
- [ ] Tauri build step produced `target/release/bundle/` artifacts.
- [ ] Artifact upload step succeeded.

### 4. Verify Android job

- [ ] Job status changes from `queued` → `in_progress` → `passed`.
- [ ] Android SDK/NDK setup completed.
- [ ] `tauri android init --ci` succeeded.
- [ ] `tauri android build --target aarch64` produced APK.
- [ ] Android release signing secrets were present and the APK signature verification step passed.
- [ ] No `*-unsigned.apk` artifact was uploaded or attached to the release.
- [ ] Artifact upload step succeeded.

### 5. Verify release publication

- [ ] `create-release` job started after both build jobs passed.
- [ ] Release object was created on the correct platform (GitHub or Gitea).
- [ ] Expected artifacts are attached:
  - `.msi` installer
  - `.exe` installer (NSIS)
  - `.apk` Android package
- [ ] Artifacts are individual files, **not** zip bundles.

### 6. Failure handling

If any job fails:
- [ ] **Do not panic.** Failed runs can be retried from the workflow UI.
- [ ] If the release object was already created but assets are missing or wrong,
  replacement assets can be re-uploaded with the same tag.
- [ ] Document the failure and retry in the release notes.

---

## Post-Release

- [ ] Download each artifact type and verify it installs/launches.
- [ ] Smoke-test the production API endpoints from a release build.
- [ ] Announce the release.
- [ ] Archive this checklist with the release notes.

---

## Known External Blockers

- Release workflow has **not** been executed end-to-end yet.
- Runner availability, signing behavior, and artifact publication are unverified.
- This checklist exists to ensure the first live run is watched, not shipped blindly.
