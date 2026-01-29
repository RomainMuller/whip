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

## Key Documents

Two documents track project state and must be consulted/updated as appropriate:

- **[ARCHITECTURE.md](../../ARCHITECTURE.md)**: Detailed architecture documentation including crate
  responsibilities, data flow diagrams, key abstractions, and extension points. **Consult this when
  exploring the codebase.** Update when making architectural changes (new crates, modified data
  flow, new abstractions).

- **[TODO.md](../../TODO.md)**: Project-level task tracking for code quality findings, technical
  debt, and future work items. **Update with any issues discovered during work**, categorized by
  priority (high/medium/low).

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
- Use **Mermaid** fenced code blocks for diagrams (`` ```mermaid ``)
- Prefer Mermaid over ASCII art when a clear diagram exists

## Commit Practices

This project follows [conventional commit](https://www.conventionalcommits.org/en/v1.0.0/)
practices.

A commit (or changeset) should contain only exactly one change, well identified, that can be
reviewed independently, and reverted independently. Proper separation of concerns is key here.

### CRITICAL: Create New Changeset BEFORE Making Edits

**When using `jj`, you MUST create a new empty changeset BEFORE making any file edits:**

```bash
# ALWAYS do this FIRST, before touching any files:
jj new -m "wip(claude): <brief intention>"

# Only THEN start editing files
```

This is non-negotiable. Editing files without first creating a new changeset pollutes unrelated
changesets and violates separation of concerns.

### Sub-Agent Commits

When implementation is delegated to sub-agents, each sub-agent should produce its own commit(s) with
appropriate description(s) and proper separation of concern. Sub-agents' work is rolled into the
main workspace by bringing these changesets as ancestors of the current working copy.

Merge commits are welcome to bring multiple independent changes together in the main trunk; the
merge commit's description should explain the broader composite feature achieved by combining the
smaller features.

## Local AI State

Plans, tasks, and workspaces are stored in `.localai/` sub-directories; see
[localai-schemas.md](./localai-schemas.md) for YAML schemas.

## Project Management

Backlog via [GitHub Issues](../../issues), tracking via [GitHub Projects](../../projects).
