# Contributing Guide

Thanks for contributing to Wallpaper Composer.

## 1. Scope and priorities
Current focus is MVP:
- image folder input
- rotating quotes from `.txt` / `.md`
- time overlay
- wallpaper update loop

Before working, read:
- `docs/PROJECT_PLAYBOOK.md`
- `docs/ARCHITECTURE.md`
- `docs/TEST_MATRIX.md`

## 2. Development setup
Required:
- Rust stable (rustup-managed)
- `rustfmt`
- `clippy`

Commands:
```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

## 3. Branch and commit style
- Use focused branches: `codex/<topic>` (or similar).
- Keep commits small and coherent.
- Reference issue IDs in commit messages when available.

Suggested commit format:
```txt
feat(cli): add init command for starter config
fix(core): handle missing HOME env var in config path resolution
docs(readme): update setup section
```

## 4. Pull request checklist
Before opening a PR:
1. Code is formatted (`cargo fmt --all`).
2. Lints pass (`cargo clippy ... -D warnings`).
3. Tests pass (`cargo test --all`).
4. User-visible changes are documented in `README.md` and/or `CHANGELOG.md`.
5. New behavior has tests where practical.

PR description should include:
- what changed
- why it changed
- how it was tested
- open risks or follow-ups

## 5. Coding guidelines
- Prefer clear, explicit code over clever code.
- Use `Result`-based error handling for fallible paths.
- Keep crate boundaries clean:
  - `wc-cli`: CLI and command dispatch
  - `wc-core`: domain logic and config logic
  - `wc-render`: rendering pipeline
- Avoid breaking user config compatibility without migration notes.

## 6. Reporting bugs
Please include:
- OS and desktop environment (GNOME/KDE/Sway, Wayland/X11)
- command used
- expected result
- actual result
- logs/output
- sample config (redacted if needed)

## 7. Security and safety
- Do not commit secrets, API tokens, private keys, or credentials.
- For potentially destructive behavior, require explicit user action.

## 8. License
By contributing, you agree your contributions are licensed under:
- `GPL-3.0-or-later` (same as this repository)

