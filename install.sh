#!/usr/bin/env bash

# Screamshot Premium Linux Installer 🚀
# Optimized for Ubuntu/Debian and modern desktop environments (X11 & Wayland).

set -euo pipefail

# Text formatting colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Decorative ASCII Art Header
echo -e "${CYAN}"
echo "========================================="
echo "   SCREAMSHOT LINUX APP INSTALLER 🚀     "
echo "========================================="
echo -e "${NC}"

# Step 1: Detect package manager and install system dependencies
echo -e "${BLUE}[1/5] Checking and installing system dependencies...${NC}"
if command -v apt-get &> /dev/null; then
    echo -e "${YELLOW}Ubuntu/Debian-based system detected. Requesting sudo to install development dependencies...${NC}"
    sudo apt-get update
    sudo apt-get install -y \
        libdbus-1-dev \
        libxdo-dev \
        libxcb-shape0-dev \
        libxcb-xfixes0-dev \
        libxkbcommon-dev \
        libgtk-3-dev \
        clang \
        pkg-config \
        build-essential
else
    echo -e "${YELLOW}Non-Debian system detected. Please ensure you have GTK3, DBus, and X11/Wayland dev libraries installed manually.${NC}"
fi

# Step 2: Ensure Cargo is installed
echo -e "\n${BLUE}[2/5] Checking for Rust and Cargo compiler...${NC}"
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Rust/Cargo is not installed.${NC}"
    echo -e "${YELLOW}Please install Rust using: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh${NC}"
    exit 1
fi
echo -e "${GREEN}Rust/Cargo compiler is available!${NC}"

# Step 3: Compile the application in release mode
echo -e "\n${BLUE}[3/5] Compiling Screamshot in release mode (highly optimized)...${NC}"
cargo build --release

# Step 4: Install the binary to ~/.local/bin
echo -e "\n${BLUE}[4/5] Installing Screamshot executable...${NC}"
BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"

echo -e "Copying compiled binary to ${CYAN}${BIN_DIR}/screamshot${NC}"
cp target/release/screamshot "$BIN_DIR/screamshot"
chmod +x "$BIN_DIR/screamshot"

# Step 5: Automatically self-register launcher
echo -e "\n${BLUE}[5/5] Performing system launcher integration...${NC}"
# Run the newly installed binary in the background for 1 second to trigger self-registration, then terminate cleanly
"$BIN_DIR/screamshot" &
PID=$!
sleep 1.5
kill $PID || true

echo -e "${GREEN}Desktop app launcher registered successfully!${NC}"

# Final completion display
echo -e "\n${GREEN}========================================="
echo "      INSTALLATION COMPLETE! 🎉          "
echo "========================================="
echo -e "${NC}"
echo -e "Screamshot is now fully installed and registered on your system."
echo -e "1. You can now close your terminal."
echo -e "2. Open your system App Launcher (press Super/Win key)."
echo -e "3. Search for ${CYAN}Screamshot${NC} and click the icon to launch!"
echo -e "4. Right-click the system tray icon to control capture features."
echo ""
