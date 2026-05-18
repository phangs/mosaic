# Screamshot 🚀

Screamshot is a premium, lightweight, and modern screen capture and scrolling-capture tool designed specifically for Linux desktop environments (X11 & Wayland). Powered by Rust, `egui`, and `xcap`, Screamshot allows users to select custom screen regions, perform dynamic scrolling captures with automatic vertical stitching, copy output to their clipboard instantly, and personalize application preferences through a sleek Settings panel.

```mermaid
graph TD
    A[System Tray Icon] -->|Right Click| B(Tray Menu)
    B -->|Capture Region| C[Overlay Region Selection]
    B -->|Capture Scrolling Region| D[Overlay + Scrolling Capture Panel]
    B -->|Settings| E[Screamshot Settings UI]
    C -->|Select & Drag| F[xcap Monitor Capture]
    D -->|Step Scroll & Capture| G[Enigo Keypress + xcap Capture]
    G -->|Stitch Frames| H[Overlap Detection & Stitching]
    F -->|Process Image| I[Save to Disk & Copy to Clipboard]
    H -->|Process Stitched Image| I
    E -->|Modify Config| J[JSON Config + Desktop Autostart]
```

---

## ✨ Features

- **🎯 Region Capture**: Interactive click-and-drag overlay supporting freeform region selection with absolute accuracy.
- **🔄 Scrolling Region Capture**: Select a custom region, perform manual step-by-step vertical scrolling, and watch the software automatically align and stitch frames into a single high-resolution image using our advanced pixel-level overlap matching algorithm.
- **⚙️ Premium Settings Manager**:
  - **Startup Autostart**: Enable/disable automatic launching on system login using Freedesktop standard desktop entries.
  - **Default Output Folder**: Customize where your screenshots land with an integrated native directory picker.
  - **Auto-Preview**: Instantly open completed captures using your system's default image viewer (`xdg-open`).
- **📋 Instant Clipboard Integration**: Images are automatically written to your system clipboard (Wayland & X11 compatible) for instant sharing.
- **🔔 Desktop Notifications**: Desktop-native notifications keep you updated on the capture's status.
- **🛡️ Intelligent Match Protection**: Safe color-mismatch guard prevents glitched stitching if unrelated contents are scrolled, fallback-appending them cleanly. Duplicate/stationary frame skipping avoids overhead.

---

## 🚀 Automatic Installation

Screamshot includes a fully-automated, interactive installation script `install.sh` that detects dependencies, updates system headers, compiles in highly-optimized release mode, moves the binary to standard user folders, and integrates the app natively into your Linux desktop launcher (GNOME, etc.).

Simply run:
```bash
./install.sh
```

---

## 🛠️ Manual Installation (Optional)

If you prefer to perform steps manually:

### 1. Install Prerequisites
```bash
sudo apt update
sudo apt install -y \
    libdbus-1-dev libxdo-dev libxcb-shape0-dev libxcb-xfixes0-dev \
    libxkbcommon-dev libgtk-3-dev clang pkg-config build-essential
```

### 2. Build & Copy Binary
```bash
cargo build --release
mkdir -p ~/.local/bin
cp target/release/screamshot ~/.local/bin/screamshot
chmod +x ~/.local/bin/screamshot
```

### 3. Register System App Launcher
Running `/home/phangs/.local/bin/screamshot` once will automatically generate its system desktop entry at `~/.local/share/applications/screamshot.desktop` for menu search availability!

---

## 🔬 Running Automated Tests
Screamshot includes a rigorous suite of automated unit tests validating overlap bounds, threshold safety guards, and frame stitching integrity.
```bash
cargo test
```

---

## 📁 Configuration Details
All application configuration preferences are safely stored in JSON format at:
`~/.config/screamshot/config.json`

Desktop autostart integration resides under:
`~/.config/autostart/screamshot.desktop`
