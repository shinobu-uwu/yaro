# Repository Guidelines

## Project Structure & Module Organization

This Cargo workspace contains a Rust 2021 freestanding kernel using `#![no_std]` and `#![no_main]`.

- `kernel/src/main.rs`: kernel entry point, Limine requests, framebuffer demonstration, and panic/halt handlers. Add kernel modules under `kernel/src/`.
- `kernel/build.rs`: selects `kernel/linker-scripts/linker-{arch}.ld` for the target architecture.
- `flake.nix` and `flake.lock`: development toolchain, kernel build, bootable ISO packaging, and QEMU launcher.
- `limine.conf`: boot menu and kernel path configuration.
- `.cargo/config.toml`: standard-library build settings.

The configured Nix build targets `x86_64-unknown-none`; additional linker scripts do not imply complete platform support. There are currently no dedicated test or asset directories.

## Build, Test, and Development Commands

Enable Nix's `flakes` and `nix-command` features, then run commands from the repository root:

- `nix develop`: enter the Rust development environment; `.envrc` also supports direnv.
- `nix build`: build the kernel ELF at `result/bin/kernel`.
- `nix build .#iso`: create the bootable image at `result/kernel.iso`.
- `nix run`: build the ISO and boot it in QEMU with UEFI firmware.
- `cargo fmt --all -- --check`: check Rust formatting inside the development shell; use `cargo fmt --all` to apply formatting.

## Coding Style & Naming Conventions

Use rustfmt defaults with four-space Rust indentation. Use `snake_case` for functions and modules, `PascalCase` for types, and `SCREAMING_SNAKE_CASE` for constants and statics. Match existing two-space indentation in Nix files.

Keep kernel code compatible with `core`; avoid `std` dependencies. Explain safety assumptions around raw pointers and inline assembly. Preserve Limine request attributes and linker sections when changing boot initialization.

## Testing Guidelines

No automated test framework or coverage threshold is configured; Nix builds disable checks with `doCheck = false`. Validate kernel changes with `nix build .#iso` and `nix run`. The current framebuffer demo should display a short white diagonal line. Record observed boot behavior and reproduction steps in the pull request.

## Commit & Pull Request Guidelines

History contains only `Initial commit`, so no established commit convention exists. Use concise, imperative subjects, such as `Add framebuffer bounds checks`, and keep commits focused.

Describe the change, motivation, and validation commands in pull requests. Link relevant issues and include screenshots for framebuffer changes. Call out target, linker, or boot configuration changes explicitly.
