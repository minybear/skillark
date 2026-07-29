# SkillArk

Universal skill manager for AI coding agents. Import skills once, manage them locally,
and safely distribute them to Claude Code, Cursor, Codex, WorkBuddy, custom agents,
or project workspaces.

## Project status

SkillArk v0.1 has completed design freeze, project scaffolding, and the first
Windows Agent discovery slice. The current M1 POC detects Claude Code, Cursor,
Codex, and WorkBuddy from local signals; Custom Agent configuration and the full
path validation matrix are next.

Start here:

- [v0.1 design index](docs/skillark/design/README.md)
- [v0.1 bootstrap plan](docs/skillark/plan/20260725-bootstrap-v0.1/00-需求概览.md)
- [open issues](docs/skillark/issues/README.md)

## Development

Prerequisites:

- Node.js 20+
- Rust stable with the MSVC toolchain
- Microsoft C++ Build Tools and Windows SDK
- Microsoft Edge WebView2

Commands:

```text
npm install
npm run check
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run tauri dev
npm run tauri build -- --debug --no-bundle
```

The React/Vite shell can be previewed with `npm run dev`. The complete desktop build
requires the [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/).
