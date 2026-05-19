# Contributing to Mosaic 🤝

First off, thank you for considering contributing to Mosaic! Contributions make the open-source community an amazing place to learn, inspire, and create.

All types of contributions are welcome: bug fixes, feature requests, documentation improvements, and architectural optimizations.

---

## 🛠️ Local Development Workflow

Mosaic is built purely in Rust. To set up your local development environment:

1. **Setup Rust**: Install Rust via [rustup](https://rustup.rs/).
2. **System Prerequisites**: Ensure GTK, X11/Wayland, and DBus headers are installed (refer to the system prerequisites in the [README.md](README.md)).
3. **Format Check**: Ensure your code conforms to the project styling standards:
   ```bash
   cargo fmt --all -- --check
   ```
4. **Linting Check**: Check for common issues or code smells using Clippy:
   ```bash
   cargo clippy --all-targets --all-features -- -D warnings
   ```
5. **Run Tests**: Verify compilation and execute unit tests:
   ```bash
   cargo test
   ```

---

## 🏗️ Code Architecture & Guidelines

Mosaic uses an event-driven design integrating `eframe`/`egui` for premium transparent UI overlays with `tray-icon` for system tray controls.

### State Flow
```rust
enum AppState {
    Hidden,                  // Active in background, tray listening
    SelectingRegion,         // Fullscreen transparent click-and-drag overlay
    SelectingScrollRegion,   // Fullscreen scrolling drag selection
    CapturingScroll,         // Scrolling step panel active, inputs simulation
    EditingSettings,         // Interactive settings menu window
}
```

### Critical Guidelines:
1. **Thread Separation**: Always run heavy computational operations (such as PNG saving, clipboard initialization, or image stitching) on separate background threads (`std::thread::spawn`) to avoid freezing the main rendering and event loops.
2. **Platform Parity**: When updating `enigo` key simulation or screenshot capturing mechanisms, ensure Wayland compatibility is preserved.
3. **No Placeholders**: Do not check in temporary assets, mock code, or partial implementations. Ensure every feature is complete and verified with unit tests.

---

## 🚀 Submitting a Pull Request

1. Fork the repository and create your branch from `master`.
2. Add comprehensive unit tests in the `tests` block for any utility functions or image-processing additions.
3. Verify your changes compile, lint cleanly, and format perfectly.
4. Open a Pull Request detailing the changes, reasoning, and visual outcome.
