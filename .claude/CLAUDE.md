# taim — Tame AI

A terminal UI application that supervises and orchestrates multiple Claude Code instances.

## Project Overview

**taim** is a Rust-based TUI that acts as an "engineering manager" for AI coding agents, spawning and coordinating multiple Claude Code subprocesses to tackle complex, parallelizable work.

## Architecture

### Workspace Structure

```
taim/
├── Cargo.toml              # Workspace root + CLI binary
├── crates/
│   ├── taim-tui/           # Ratatui-based terminal UI
│   ├── taim-session/       # Claude Code subprocess management
│   ├── taim-protocol/      # Message types, events, shared contracts
│   └── taim-config/        # Configuration loading & validation
└── tests/                  # Integration tests
```

### Crate Responsibilities

| Crate | Purpose |
|-------|---------|
| `taim` (root) | CLI entrypoint, arg parsing, orchestration |
| `taim-tui` | UI components, layout, input handling, rendering |
| `taim-session` | Spawn/manage Claude Code processes, I/O streaming |
| `taim-protocol` | Shared types (events, messages, errors) — no I/O |
| `taim-config` | Config file parsing, defaults, validation |

## Technology Stack

- **Rust Edition**: 2024
- **Async Runtime**: Tokio
- **TUI Framework**: Ratatui + crossterm
- **Serialization**: serde + serde_json
- **Error Handling**: thiserror (library crates), anyhow (binary)
- **CLI Parsing**: clap (derive)
- **Testing**: built-in + proptest + insta

## Project Management

- **Backlog**: Managed via [GitHub Issues](../../issues)
- **Tracking**: Use [GitHub Projects](../../projects) to track issue state (Backlog → In Progress → Done)
- **Labels**: Use labels to categorize issues (bug, enhancement, documentation, etc.)

## Local state management

- **Current plans**: stored in the `.localai/plans/` directory at crate root
    - Plans are to be updated as implementation progresses so they stay up-to date
    - Plans are presented as markdown documents; with a YAML front-matter providing high-level information on the intent
    - Front-matter schema for plans:
      ```yaml
      ---
      id: plan-NNN              # Unique identifier (e.g., plan-001)
      title: Short Title        # Human-readable title
      status: draft|approved|in_progress|completed|abandoned
      created: YYYY-MM-DD       # Creation date
      tags: [tag1, tag2]        # Categorization tags
      priority: low|medium|high # Priority level
      estimated_effort: small|medium|large|xlarge
      ---
      ```
- **Detailed tasks**: stored in the `.localai/tasks/` directory at crate root
    - Tasks are created from the plans, to fan out work to separate agents
    - Tasks are presented as markdown documents; with a YAML front-matter linking it to the overarching plan
    - They contain enough information for the separate agent to independently work on implementation
    - Sub-agents are to update their assigned task as they progress, including marking sub-tasks as done, etc..
    - Front-matter schema for tasks:
      ```yaml
      ---
      id: task-NNN              # Unique identifier (e.g., task-001)
      plan: plan-NNN            # Reference to parent plan
      title: Short Title        # Human-readable title
      status: pending|in_progress|completed|blocked
      assigned_to: agent-type   # Which agent type handles this
      created: YYYY-MM-DD       # Creation date
      dependencies: [task-NNN]  # Tasks that must complete first
      ---
      ```
- **Workspaces**: when spinning sub-agents to work concurrently on coding tasks, use the appropriate tool to manage
  (create/delete) workspaces in the `.localai/workspaces` directory at the crate root; ensuring you always:
    - Retrofit the changes as individual commits into the root/default workspace once done
    - Clean up (i.e, delete) the workspaces after they are no longer needed
    - Have commits be siblings of each others unless the child depends on changes in the parent (introduce merge commits
      using conventional commit to describe it)

## Code Style & Conventions

### General

- **Idiomatic Rust**: Prefer standard library types; avoid unnecessary dependencies
- **No `unwrap()` in library code**: Use `?` or explicit error handling
- **`#[must_use]`** on functions returning values that shouldn't be ignored
- **Documentation**: All public items must have doc comments with examples where appropriate

### Naming

- Types: `PascalCase`
- Functions/methods: `snake_case`
- Constants: `SCREAMING_SNAKE_CASE`
- Crate names: `taim-*` (kebab-case)
- Module files: `snake_case.rs`

### Error Handling

- Each crate defines its own `Error` type via `thiserror`
- Root binary uses `anyhow::Result` for ergonomic error composition
- Errors should be descriptive and actionable

```rust
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("failed to spawn Claude Code process: {0}")]
    SpawnFailed(#[source] std::io::Error),

    #[error("session {id} terminated unexpectedly with code {code}")]
    UnexpectedTermination { id: SessionId, code: i32 },
}
```

### Async Patterns

- Use `tokio::spawn` for concurrent tasks
- Prefer channels (`tokio::sync::mpsc`) for inter-component communication
- Use `tokio::select!` for multiplexing; always handle cancellation gracefully
- Avoid blocking the async runtime — offload CPU-heavy work to `spawn_blocking`

### TUI Patterns (taim-tui)

- Separate **state** (data) from **view** (rendering)
- Use message-passing for state updates (Elm-ish architecture)
- Keep render functions pure: `fn render(state: &State, frame: &mut Frame)`
- Handle resize events gracefully

## Testing Strategy

### Unit Tests

- Colocated with code (`#[cfg(test)]` modules)
- Test pure logic extensively
- Mock I/O boundaries

### Property-Based Tests (proptest)

- Use for parsers, serialization round-trips, state machines
- Place in `tests/` or dedicated `proptest` modules

```rust
proptest! {
    #[test]
    fn config_roundtrip(config: Config) {
        let json = serde_json::to_string(&config)?;
        let parsed: Config = serde_json::from_str(&json)?;
        prop_assert_eq!(config, parsed);
    }
}
```

### Snapshot Tests (insta)

- Use for TUI rendering, CLI output, complex struct serialization
- Review snapshots carefully before accepting

```rust
#[test]
fn test_help_output() {
    let output = Command::new("taim").arg("--help").output().unwrap();
    insta::assert_snapshot!(String::from_utf8_lossy(&output.stdout));
}
```

### Integration Tests

- Place in `tests/` directory
- Test cross-crate interactions
- Use `tokio::test` for async tests

## Commands

```bash
# Build
cargo build --workspace

# Run
cargo run -- [args]

# Test all
cargo test --workspace

# Test specific crate
cargo test -p taim-session

# Clippy (treat warnings as errors)
cargo clippy --workspace -- -D warnings

# Format check
cargo fmt --check

# Update snapshots
cargo insta review
```

## Dependencies Policy

- **Workspace-level dependencies**: All dependencies must be declared in the root `Cargo.toml` under `[workspace.dependencies]`. Individual crates reference them via `dependency.workspace = true`.
- **Version pinning**: Pin dependencies to at least the minor version (e.g., `"1.40"` not `"1"`) to ensure all required features are available and builds are reproducible.
- Prefer well-maintained crates with > 1M downloads or strong ecosystem presence
- Audit new dependencies for security and maintenance status
- Use `cargo update` deliberately to bump versions

Example in root `Cargo.toml`:
```toml
[workspace.dependencies]
tokio = { version = "1.40", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

Example in crate `Cargo.toml`:
```toml
[dependencies]
tokio = { workspace = true }
serde = { workspace = true }
```

## AI Collaboration Notes

This codebase is largely AI-generated. When contributing:

- Maintain consistency with existing patterns
- Don't introduce new architectural patterns without discussion
- Prefer explicit over clever
- When in doubt, add a test
