English | [Português (Brasil)](setup-android.pt-BR.md)

# 🤖 Android Environment Setup Guide (Debian / Linux)

This guide describes the steps required to set up a complete Android development environment for projects scaffolded with **jirau**, including building the native **Rust** core via `cargo-ndk`.

The versions below match the pins in [`src/templates/versions.rs`](../src/templates/versions.rs) (template 0.4.0). Every generated project also ships its own `docs/setup-android.md` with the same pins.

---

## 1. Installing the JDK and System Dependencies

On Debian GNU/Linux (or derivatives such as Ubuntu/Mint), install the essential libraries and JDK 25 (the current LTS). Generated projects pin JDK 25 and Rust 1.99.1 in `.mise.toml`, so the easiest path is [`mise`](https://mise.jdx.dev/):

```bash
sudo apt update
sudo apt install -y curl git unzip libglu1-mesa
mise install   # inside a generated project: JDK 25 and Rust 1.99.1, as defined in .mise.toml
```

### Hardware Acceleration for the Emulator (KVM)

To run Android emulators with hardware acceleration:

```bash
sudo apt install -y qemu-kvm
sudo usermod -aG kvm $USER
```
*(Restart your session or log out/in to apply the new group.)*

---

## 2. Headless Servers (Debian 13 CLI / No Graphical Interface)

On servers or headless environments (no desktop environment), **you do not need to install Android Studio**. The environment is managed 100% from the command line with `mise`, `sdkmanager` and `cargo-ndk`:

### 2.1. Install the Android SDK Command-line Tools

```bash
mkdir -p ~/Android/Sdk/cmdline-tools
curl -L -o /tmp/cmdline-tools.zip https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip
python3 -m zipfile -e /tmp/cmdline-tools.zip /tmp/cmdline-extracted
mv /tmp/cmdline-extracted/cmdline-tools ~/Android/Sdk/cmdline-tools/latest
chmod +x ~/Android/Sdk/cmdline-tools/latest/bin/*
rm -rf /tmp/cmdline-tools.zip /tmp/cmdline-extracted
```

### 2.2. Accept Licenses and Install Components (SDK, NDK, Platform-tools)

```bash
# Accept all SDK licenses
yes | ~/Android/Sdk/cmdline-tools/latest/bin/sdkmanager --sdk_root=$HOME/Android/Sdk --licenses

# Install the required packages
~/Android/Sdk/cmdline-tools/latest/bin/sdkmanager --sdk_root=$HOME/Android/Sdk \
  "platform-tools" \
  "platforms;android-37" \
  "build-tools;37.0.0" \
  "ndk;30.0.16248370"
```

CMake is not required: the Rust core is built with `cargo-ndk`, not with an Android CMake project.

---

## 3. Workstations with a Graphical Interface (Android Studio)

If you are on a local machine with a desktop (GNOME, KDE, etc.) and want to use the IDE:

### Option A: Flatpak (Recommended for isolation)

```bash
flatpak install flathub com.google.AndroidStudio
flatpak run com.google.AndroidStudio
```

### Option B: Official Tarball

1. Download the stable `.tar.gz` from [developer.android.com/studio](https://developer.android.com/studio).
2. Extract it to `/opt`:
   ```bash
   sudo tar -xvzf android-studio-*.tar.gz -C /opt/
   ```
3. Start the installer:
   ```bash
   /opt/android-studio/bin/studio.sh
   ```

---

## 4. Required SDK & NDK Components

If you set things up through the Android Studio interface:
**Settings** $\rightarrow$ **Languages & Frameworks** $\rightarrow$ **Android SDK** $\rightarrow$ **SDK Tools** tab:
- [x] **Android SDK Command-line Tools (latest)**
- [x] **Android SDK Platform-Tools** (`adb`)
- [x] **NDK (Side by side)** (`30.0.16248370`)

By default on Linux, the SDK is installed in `~/Android/Sdk`.

---

## 5. Environment Variables

Set these variables so that command-line tools (`adb`, `cargo-ndk`, `gradlew`) can find the SDK.

### Bash (`~/.bashrc`) or Zsh (`~/.zshrc`)

```bash
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/30.0.16248370"
export PATH="$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$HOME/.cargo/bin:$PATH"
```

### Fish Shell (`~/.config/fish/config.fish`)

```fish
set -gx ANDROID_HOME "$HOME/Android/Sdk"
set -gx ANDROID_SDK_ROOT "$ANDROID_HOME"
set -gx ANDROID_NDK_HOME "$ANDROID_HOME/ndk/30.0.16248370"
fish_add_path "$ANDROID_HOME/cmdline-tools/latest/bin" "$ANDROID_HOME/platform-tools" "$HOME/.cargo/bin"
```

After editing, reload your shell:
```bash
source ~/.bashrc # or source ~/.zshrc
```

---

## 6. Rust Toolchain for Android (`cargo-ndk` and Targets)

The Rust compiler needs the Android targets installed:

```bash
# Add the Rust cross-compilation targets
rustup target add \
  aarch64-linux-android \
  armv7-linux-androideabi \
  x86_64-linux-android \
  i686-linux-android

# Install the cargo-ndk utility
cargo install cargo-ndk
```

### Automated Script

jirau ships a script that checks and configures these dependencies automatically:

```bash
./scripts/setup-android-env.sh
```

---

## 7. How the Rust Core Gets Into the APK

Generated projects build the Rust core with `scripts/build-android-ffi.sh` (`cargo-ndk` → `android/app/src/main/jniLibs/` + Kotlin bindings in `android/app/build/generated/uniffi`). Gradle runs this script automatically in `preBuild`, so `jirau run`, `./gradlew`, Android Studio and CI all follow the same flow.

By default the `arm64-v8a` and `x86_64` ABIs are built. To build only one ABI (for example, for the emulator):

```bash
cd android && ./gradlew assembleDebug -Pjirau.rustAbis=x86_64
```

`jirau run android` detects the ABI of the connected device and builds only that one.

---

## 8. Verifying the Environment with jirau doctor

To check that everything is configured correctly:

```bash
jirau doctor
```
Or from the repository root:
```bash
make doctor
```
