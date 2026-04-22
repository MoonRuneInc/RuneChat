# Android Testing Setup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Set up the Android development toolchain on WSL2 so Cauldron can be built, run, and iterated on against a physical Android device locally.

**Architecture:** Install JDK 17 + Android SDK + NDK 27 in WSL2, add Rust Android cross-compilation targets, initialize the Tauri Android project, and connect a physical device via wireless ADB. No application code changes — this is purely toolchain setup. The existing CI workflow (`build.yml` `tauri-android` job) is the canonical reference for required versions.

**Tech Stack:** Tauri 2, Android SDK (API 35, minSdk 24), NDK 27.0.11902837, Rust aarch64-linux-android target, ADB over TCP (wireless debugging)

---

## Phase 0: Quick Smoke Test via CI (do this first, no setup required)

Before spending time on local toolchain, validate the app actually builds and runs on Android.

### Task 0: Dispatch CI Android build and sideload APK

**Files:** none

- [ ] **Step 1: Trigger the workflow dispatch**

  Go to `https://github.com/MoonRune/Cauldron/actions/workflows/build.yml` → click **Run workflow** → select branch `dev` → Run.

  The `tauri-android` job builds an APK. Wait for it to complete (10–15 min).

- [ ] **Step 2: Download the APK artifact**

  On the completed run page, download the artifact named `cauldron-android` (or similar). Unzip it — you'll have a `.apk` file.

- [ ] **Step 3: Enable Developer Options on your Android device**

  Settings → About phone → tap **Build number** 7 times → go back to Settings → Developer options → enable **USB debugging** and **Install via USB**.

  Alternatively for wireless (Android 11+): Developer options → **Wireless debugging** → enable.

- [ ] **Step 4: Install the APK**

  If using USB:
  ```bash
  adb devices           # confirm device appears
  adb install cauldron.apk
  ```

  If sideloading manually: copy the APK to your device and open it in Files to install (requires "Install unknown apps" permission for your file manager).

- [ ] **Step 5: Verify the app launches**

  Open Cauldron on the device. It should show the login screen and connect to `https://chat.moonrune.cc` (or localhost if you modify the build config — see note below).

  > **Note:** The CI build hardcodes `VITE_API_BASE_URL=https://chat.moonrune.cc/api`. For testing against your local stack, you'll need a local dev build (Phase 2 below) or to temporarily modify those env vars before dispatch.

---

## Phase 1: Install Android Toolchain on WSL2

### Task 1: Install JDK 17

**Files:**
- Modify: `~/.bashrc` (add JAVA_HOME)

- [ ] **Step 1: Install Temurin JDK 17**

  ```bash
  sudo apt update
  sudo apt install -y wget apt-transport-https gnupg
  wget -qO - https://packages.adoptium.net/artifactory/api/gpg/key/public | sudo gpg --dearmor -o /usr/share/keyrings/adoptium.gpg
  echo "deb [signed-by=/usr/share/keyrings/adoptium.gpg] https://packages.adoptium.net/artifactory/deb $(lsb_release -cs) main" | sudo tee /etc/apt/sources.list.d/adoptium.list
  sudo apt update
  sudo apt install -y temurin-17-jdk
  ```

- [ ] **Step 2: Verify**

  ```bash
  java -version
  ```
  Expected: `openjdk version "17.x.x" ...`

- [ ] **Step 3: Set JAVA_HOME**

  ```bash
  echo 'export JAVA_HOME=$(dirname $(dirname $(readlink -f $(which java))))' >> ~/.bashrc
  source ~/.bashrc
  echo $JAVA_HOME
  ```
  Expected: a path like `/usr/lib/jvm/temurin-17-amd64`

---

### Task 2: Install Android SDK command-line tools

**Files:**
- Create: `~/android-sdk/` (SDK root)
- Modify: `~/.bashrc` (add ANDROID_HOME, PATH)

- [ ] **Step 1: Download the latest cmdline-tools**

  ```bash
  mkdir -p ~/android-sdk/cmdline-tools
  cd ~/android-sdk/cmdline-tools
  wget https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip -O cmdline-tools.zip
  unzip cmdline-tools.zip
  mv cmdline-tools latest
  rm cmdline-tools.zip
  ```

- [ ] **Step 2: Set ANDROID_HOME and update PATH**

  ```bash
  cat >> ~/.bashrc << 'EOF'
  export ANDROID_HOME=$HOME/android-sdk
  export PATH=$PATH:$ANDROID_HOME/cmdline-tools/latest/bin
  export PATH=$PATH:$ANDROID_HOME/platform-tools
  EOF
  source ~/.bashrc
  ```

- [ ] **Step 3: Verify sdkmanager is available**

  ```bash
  sdkmanager --version
  ```
  Expected: a version number like `13.0`

- [ ] **Step 4: Accept licenses**

  ```bash
  yes | sdkmanager --licenses
  ```
  Expected: all license prompts accepted.

---

### Task 3: Install Android platform, build-tools, NDK, and ADB

**Files:** none (SDK packages installed to `~/android-sdk/`)

- [ ] **Step 1: Install required SDK packages**

  ```bash
  sdkmanager "platforms;android-35" "build-tools;35.0.0" "platform-tools" "ndk;27.0.11902837"
  ```
  This downloads ~2 GB. Wait for completion.

- [ ] **Step 2: Set NDK_HOME**

  ```bash
  echo 'export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/27.0.11902837' >> ~/.bashrc
  source ~/.bashrc
  echo $ANDROID_NDK_HOME
  ```
  Expected: `~/.../android-sdk/ndk/27.0.11902837`

- [ ] **Step 3: Verify ADB is available**

  ```bash
  adb version
  ```
  Expected: `Android Debug Bridge version 1.x.x`

---

### Task 4: Add Rust Android cross-compilation targets

**Files:** none

- [ ] **Step 1: Install Rust Android targets**

  ```bash
  rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
  ```

- [ ] **Step 2: Verify**

  ```bash
  rustup target list --installed | grep android
  ```
  Expected output (all four lines):
  ```
  aarch64-linux-android
  armv7-linux-androideabi
  i686-linux-android
  x86_64-linux-android
  ```

---

## Phase 2: Initialize Project and Connect Device

### Task 5: Initialize the Tauri Android project

**Files:**
- Create: `frontend/src-tauri/gen/android/` (generated Android Studio project)

- [ ] **Step 1: Run tauri android init**

  ```bash
  cd /home/mystiatech/projects/cc/moonrune/Cauldron/frontend
  ANDROID_HOME=$HOME/android-sdk NDK_HOME=$HOME/android-sdk/ndk/27.0.11902837 npx tauri android init
  ```
  Expected: `frontend/src-tauri/gen/android/` directory is created with a Gradle project inside.

- [ ] **Step 2: Verify the generated project structure**

  ```bash
  ls frontend/src-tauri/gen/android/
  ```
  Expected: `app/`, `buildSrc/`, `gradle/`, `build.gradle.kts`, `settings.gradle.kts` (or similar Gradle files).

- [ ] **Step 3: Commit the generated Android project**

  ```bash
  cd /home/mystiatech/projects/cc/moonrune/Cauldron
  git add frontend/src-tauri/gen/android/
  git commit -m "feat(android): initialize Tauri Android project"
  ```

---

### Task 6: Connect a physical device via wireless ADB

> **Why wireless?** WSL2 cannot access USB devices directly without `usbipd-win`. Wireless ADB is simpler and works out of the box on Android 11+.

**Files:** none

- [ ] **Step 1: Enable Wireless Debugging on the device**

  On your Android device (Android 11+):
  Settings → About phone → tap Build number 7 times to unlock Developer options → back to Settings → System → Developer options → enable **Wireless debugging**.

  Tap **Wireless debugging** to open it. Leave the screen open.

- [ ] **Step 2: Pair ADB with the device**

  In the Wireless debugging screen, tap **Pair device with pairing code**. Note the IP address, pairing port, and 6-digit code shown.

  In WSL2:
  ```bash
  adb pair <device-ip>:<pairing-port>
  ```
  Enter the 6-digit code when prompted.
  Expected: `Successfully paired to <ip>:<port>`

- [ ] **Step 3: Connect ADB**

  Back in Wireless debugging screen on your device, note the IP and port shown at the top (different from the pairing port).

  ```bash
  adb connect <device-ip>:<port>
  ```
  Expected: `connected to <ip>:<port>`

- [ ] **Step 4: Verify device is visible**

  ```bash
  adb devices
  ```
  Expected:
  ```
  List of devices attached
  <ip>:<port>    device
  ```

---

## Phase 3: Run Dev Build on Device

### Task 7: Build and run Cauldron on device

**Files:** none (no code changes — this validates the toolchain)

- [ ] **Step 1: Start the local stack**

  In a separate terminal:
  ```bash
  cd /home/mystiatech/projects/cc/moonrune/Cauldron
  docker compose up -d
  ```
  Wait until `curl http://localhost:8080/health` returns `{"status":"ok"}`.

- [ ] **Step 2: Run tauri android dev**

  ```bash
  cd /home/mystiatech/projects/cc/moonrune/Cauldron/frontend
  ANDROID_HOME=$HOME/android-sdk NDK_HOME=$HOME/android-sdk/ndk/27.0.11902837 npm run tauri:android:dev
  ```

  Tauri will build the Rust Android library, build the Vite frontend, package the APK, install it on the connected device, and open it. Hot-reload is active — changes to `frontend/src/` will update the app on the device.

  Expected: The app opens on your Android device showing the Cauldron login screen. Network requests go to your local stack via the Vite dev server proxy (port 8080 on the WSL2 host).

  > **Note on networking:** The Vite proxy runs in WSL2 at `localhost:5173`, but the Android device is on the same WiFi. For the dev build to reach your local stack, you may need to forward WSL2's port 5173 (and 8080) to your Windows host IP. If the app shows a connection error, run this on Windows:
  > ```
  > netsh interface portproxy add v4tov4 listenport=5173 listenaddress=0.0.0.0 connectport=5173 connectaddress=<WSL2-IP>
  > netsh interface portproxy add v4tov4 listenport=8080 listenaddress=0.0.0.0 connectport=8080 connectaddress=<WSL2-IP>
  > ```
  > Then in `frontend/src-tauri/tauri.conf.json`, update the dev URL to use your Windows host IP instead of localhost.

- [ ] **Step 3: Smoke test the golden path**

  On the device:
  1. Register a new account (use an invite link from your local stack)
  2. Create a server
  3. Send a message in a channel
  4. Verify the message appears in real-time on the browser client at `http://localhost:5173` open in another window

- [ ] **Step 4: Commit env var setup to dev notes**

  Add a note to `CLAUDE.md` under Development Commands for Android:
  ```bash
  cd /home/mystiatech/projects/cc/moonrune/Cauldron
  git add CLAUDE.md
  git commit -m "docs: add Android dev commands to CLAUDE.md"
  ```

---

## Persistent Environment (Optional Cleanup)

If you don't want to prefix every `npm run tauri:android:*` command with the env vars, add them permanently:

```bash
# already done in Tasks 2–3, just confirming ~/.bashrc has all of these:
export ANDROID_HOME=$HOME/android-sdk
export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/27.0.11902837
export PATH=$PATH:$ANDROID_HOME/cmdline-tools/latest/bin
export PATH=$PATH:$ANDROID_HOME/platform-tools
```

Then `source ~/.bashrc` and the commands run without prefixes.
