# whip

TUI application that orchestrates multiple Claude Code instances as an "engineering manager" for AI
coding agents.

## Quick Reference

```bash
cargo build --workspace          # Build
cargo test --workspace           # Test all
cargo test -p whip-tui           # Test specific crate
cargo clippy --workspace -- -D warnings  # Lint
cargo fmt --check                # Format check
cargo insta review               # Update snapshots
```

## Architecture

```
whip/
├── Cargo.toml         # Workspace root + CLI binary
├── crates/
│   ├── tui/           # Ratatui-based terminal UI (whip-tui)
│   └── protocol/      # Shared types, events, errors — no I/O (whip-protocol)
└── tests/             # Integration tests
```

**Tech stack**: Rust 2024, Tokio, Ratatui + crossterm, serde, thiserror/anyhow, clap, proptest,
insta

## Code Patterns

### Error Handling

- Library crates: Define `Error` enum via `thiserror`, descriptive messages
- Binary: Use `anyhow::Result`
- **No `unwrap()` in library code** — use `?` or explicit handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("failed to spawn process: {0}")]
    SpawnFailed(#[source] std::io::Error),
}
```

### Async

- `tokio::spawn` for concurrency, `mpsc` channels for communication
- `tokio::select!` for multiplexing — handle cancellation gracefully
- CPU-heavy work → `spawn_blocking`

### TUI (whip-tui)

- Elm-ish: separate state from view, message-passing for updates
- Pure render: `fn render(state: &State, frame: &mut Frame)`

## Testing

| Type                | Location               | Use For                            |
| ------------------- | ---------------------- | ---------------------------------- |
| Unit                | `#[cfg(test)]` modules | Pure logic                         |
| Property (proptest) | `tests/`               | Parsers, serialization round-trips |
| Snapshot (insta)    | `tests/`               | TUI rendering, CLI output          |
| Integration         | `tests/`               | Cross-crate, use `#[tokio::test]`  |

## Dependencies

All deps in root `Cargo.toml` under `[workspace.dependencies]`. Crates use `dep.workspace = true`.
Pin to minor version minimum (e.g., `"1.40"` not `"1"`).

## Conventions

- `#[must_use]` on functions returning values that shouldn't be ignored
- Doc comments on all public items
- Prefer std library types; avoid unnecessary deps
- Prefer explicit over clever

### Documentation

- Markdown files use **GFM** (GitHub Flavored Markdown)
- Use **Mermaid** fenced code blocks for diagrams (` ```mermaid `)
- Prefer Mermaid over ASCII art when a clear diagram exists

## Local AI State

Plans, tasks, and workspaces are stored in `.localai/` sub-directories; see
[localai-schemas.md](./localai-schemas.md) for YAML schemas.

## Project Management

Backlog via [GitHub Issues](../../issues), tracking via [GitHub Projects](../../projects).
