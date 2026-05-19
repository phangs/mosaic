# Screamshot Release Guide

This document outlines the step-by-step procedure to compile, optimize, package, and distribute highly optimized production releases of **Screamshot** on Linux systems.

---

## 📋 Prerequisites & Dependencies

To build a production-ready release, ensure that the necessary build systems and system libraries are installed on the host machine.

### On Debian/Ubuntu-based Systems:
```bash
sudo apt update
sudo apt install -y build-essential pkg-config libgtk-3-dev libx11-dev libxtst-dev libwayland-dev libxkbcommon-dev
```

### On Fedora/RedHat-based Systems:
```bash
sudo dnf groupinstall "Development Tools"
sudo dnf install -y pkg-config gtk3-devel libX11-devel libXtst-devel wayland-devel libxkbcommon-devel
```

### On Arch Linux:
```bash
sudo pacman -Syu --needed base-devel pkgconf gtk3 libxtst wayland libxkbcommon
```

---

## 🚀 Building the Release Binary

To generate the most performant, optimized, and stripped executable, execute the following commands in the project root:

### 1. Compile in Release Mode
Run the compiler with high-optimization flags:
```bash
cargo build --release
```
> [!NOTE]
> The compiled binary will be placed at `target/release/screamshot`.

### 2. Strip Debug Symbols (Highly Recommended)
Rust release builds still contain substantial debugging symbols. Stripping them reduces the binary footprint significantly (typically from **~40MB down to ~3MB**):
```bash
strip target/release/screamshot
```

### 3. Verification
Verify the build is fully operational:
```bash
./target/release/screamshot
```

---

## 📦 Packaging & Installation (Linux Desktop Integration)

To integrate `screamshot` natively into your Linux desktop environment, follow these steps to install the executable, set up launcher icons, and configure keybindings.

### 1. Install the Executable
Move the stripped executable to a standard user bin path:
```bash
sudo cp target/release/screamshot /usr/local/bin/
```

### 2. Install Desktop Launcher
Create a beautiful desktop launcher file so you can search and start `screamshot` from your application menu.

Create `/usr/share/applications/screamshot.desktop` (System-wide) or `~/.local/share/applications/screamshot.desktop` (User-specific):

```ini
[Desktop Entry]
Name=Screamshot
Comment=Premium scrolling and region screenshot utility
Exec=/usr/local/bin/screamshot
Icon=screenshot
Terminal=false
Type=Application
Categories=Utility;Graphics;
StartupNotify=true
```

Update desktop database to register the entry:
```bash
update-desktop-database ~/.local/share/applications/
```

### 3. Autostart Integration
`screamshot` is designed to run silently in the system tray. To launch it automatically upon desktop login, copy the desktop entry to your autostart directory:
```bash
mkdir -p ~/.config/autostart
cp ~/.local/share/applications/screamshot.desktop ~/.config/autostart/
```

---

## 🛠 Troubleshooting & Optimization Flags

If you want to squeeze even more performance or compile for minimal binary sizes, you can customize the compilation profile in `Cargo.toml`.

Add the following to your `Cargo.toml` if it isn't already present:

```toml
[profile.release]
opt-level = 3          # Maximize speed/performance optimizations
lto = true             # Enable Link-Time Optimization across all crates
codegen-units = 1      # Reduce parallel codegen units to improve LTO optimization
panic = "abort"        # Eliminate stack unwinding tables for smaller binary footprint
strip = true           # Automatically strip symbols during build (requires Rust 1.59+)
```
