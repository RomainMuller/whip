# TODO

## Code Quality

This section documents code quality findings from a review of the workspace crates. Items are
categorized by type and prioritized by impact (high/medium/low).

### High Priority

#### Missing Crates

- [ ] **whip-session**: The `whip-session` crate is referenced in documentation but not yet
      implemented. This crate is essential for the core functionality of spawning and managing
      Claude Code subprocesses.
- [x] **whip-config**: ~~The `whip-config` crate is referenced in documentation but not yet
      implemented.~~ Implemented with JSON5 config file support, repository management, polling
      configuration, and GitHub token resolution.

#### Error Handling

- [ ] **whip-tui uses anyhow in library crate**: The TUI crate is a library (`whip-tui`) but uses
      `anyhow` for error handling (see `Cargo.toml` dependencies and `app.rs:run()` return type).
      Per project conventions, library crates should use `thiserror` with custom error types.
      Consider creating a `TuiError` enum.
- [ ] **terminal::setup_terminal returns TerminalError but app.run returns anyhow::Result**: The
      `terminal` module correctly uses `thiserror::Error` for `TerminalError`, but the `App::run`
      method returns `anyhow::Result<()>`. This inconsistency should be unified.

### Medium Priority

#### Missing `#[must_use]` Attributes

- [ ] **Lane::remove_task**: Returns `Option<Task>` but lacks `#[must_use]` - callers might
      accidentally discard the removed task.
- [ ] **KanbanBoard::move_task**: Returns `bool` indicating success but lacks `#[must_use]` -
      callers should check if the move succeeded.
- [ ] **KanbanBoard::remove_task**: Returns `Option<Task>` but lacks `#[must_use]`.
- [ ] **AppState::dismiss_help**: Returns `bool` but lacks `#[must_use]`.
- [ ] **event::poll_event**: Returns `std::io::Result<Option<Event>>` but lacks `#[must_use]`.
- [ ] **state_color_bright in task_card.rs**: Private function that returns `Color` but lacks
      `#[must_use]` (minor, internal only).

#### Documentation Gaps

- [ ] **Lane::get_task_mut**: Missing documentation example section.
- [ ] **KanbanBoard::get_task_mut**: Missing documentation example section.
- [ ] **KanbanBoard::lane_mut**: Missing documentation example section.
- [ ] **AppState**: Most mutation methods (navigate__, toggle__, scroll_*) lack documentation
      examples.
- [ ] **event module**: The `poll_event` function lacks usage examples in doc comments.
- [ ] **widgets::markdown**: Public functions like `render_markdown` have good docs but consider
      adding more complex usage examples.

#### API Ergonomics

- [ ] **Inconsistent navigation wrapping**: `navigate_left`/`navigate_right` wrap around (lane 0 ->
      lane 3), but `navigate_up`/`navigate_down` also wrap. This is consistent but may surprise
      users expecting directional navigation without wrap on the first/last task.
- [ ] **detail_scroll is u16 but scroll_detail takes i16**: The type mismatch requires casting and
      could be error-prone. Consider using a consistent signed type internally or providing separate
      `scroll_up` and `scroll_down` methods.

#### Test Coverage

- [ ] **App::view is untested**: The main rendering function `App::view` is only tested indirectly
      through snapshot tests. Consider unit tests for layout edge cases.
- [ ] **App::handle_click edge cases**: Click handling is tested but not exhaustively for boundary
      conditions (clicks exactly on borders, zero-width lanes).
- [ ] **markdown module edge cases**: The markdown renderer has excellent test coverage but could
      benefit from fuzzing with `cargo-fuzz` given the complexity of parsing.

### Low Priority

#### Async Patterns

- [ ] **App::run is async but doesn't use async features**: The main loop in `App::run` is async but
      only uses synchronous `poll_event`. This is fine for now but will need refactoring when adding
      actual async session management.
- [ ] **No cancellation handling**: The main loop doesn't handle graceful shutdown via tokio
      signals. Consider adding `tokio::signal::ctrl_c()` handling for cleaner shutdown.

#### Potential Optimizations

- [ ] **markdown render_table allocates frequently**: Table rendering in the markdown module creates
      many intermediate `String` allocations. For performance-critical paths, consider using `Cow`
      or pre-allocated buffers.
- [ ] **Lane::remove_task is O(n)**: Task removal searches linearly. For large lane sizes, consider
      using an indexed structure or HashMap for task lookup.

### Architecture Notes

#### Strengths

- Excellent separation of concerns between `whip-protocol` (pure data types) and `whip-tui` (UI
  logic)
- Comprehensive test coverage with unit tests, property-based tests (proptest), and snapshot tests
  (insta)
- Good use of `thiserror` in library code (protocol crate)
- Clean Elm-ish architecture with message-passing in the TUI
- Thorough documentation with examples on most public items

#### Areas for Future Consideration

- The TUI crate would benefit from a proper error type hierarchy when session management is added
- Consider separating widget rendering into pure functions that return `impl Widget` rather than
  taking `&mut Buffer` directly for better composability
- The markdown renderer is quite complex - consider extracting table rendering to a separate module
