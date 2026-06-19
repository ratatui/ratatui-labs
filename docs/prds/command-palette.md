# Ratatui Command Palette PRD

## Status

Experimental.

This document describes a command palette experiment for `ratatui-labs`. It is not a commitment to
a published crate, a stable API, or inclusion in Ratatui core.

Accepted tradeoffs are recorded in [architecture decision records](../adrs/):

- [0001 Command Palette Crate Shape](../adrs/0001-command-palette-crate-shape.md)
- [0002 Action API Construction And Field Visibility](../adrs/0002-action-api-construction-and-field-visibility.md)
- [0003 Owned Palette View Data](../adrs/0003-owned-palette-view-data.md)
- [0004 Betamax Rendered Validation](../adrs/0004-betamax-rendered-validation.md)

The goal is to test whether Ratatui applications benefit from a reusable action model plus command
palette presentation, and to identify the smallest API that could justify keeping focused crates
such as `ratatui-action` and `ratatui-command-palette`.

## Background

Many Ratatui applications bind actions directly to key events:

```rust
match key {
    KeyCode::Char('q') => app.quit(),
    KeyCode::Char('/') => app.search(),
    KeyCode::Char('t') => app.switch_theme(),
    _ => {}
}
```

This works for small applications, but it makes actions hard to reuse across keybindings, help
screens, menus, command palettes, toolbars, context menus, onboarding flows, and automation.

A command palette is a useful UI surface, but the deeper experiment is an action abstraction that
can be rendered and invoked from multiple places.

## Problem

Ratatui does not currently provide a shared model for describing application actions independently
from the UI surface that invokes them.

As a result, applications often duplicate metadata such as title, description, shortcut,
enabled/disabled state, category, argument requirements, and preview behavior across keybindings,
help text, menus, and ad hoc command palettes.

## Goals

1. Define a small experimental action model.
1. Build a command palette state machine over that model.
1. Keep application state ownership outside the palette.
1. Allow multiple layouts and renderers.
1. Support command discovery through filtering or search.
1. Support optional previews.
1. Support simple argument collection.
1. Produce at least one example application that exercises the API.
1. Keep the experiment small enough to delete or redesign.

## Non-goals

- Do not design a full Ratatui application framework.
- Do not require apps to adopt a component architecture.
- Do not require async.
- Do not require serialization in the first version.
- Do not build natural-language command execution.
- Do not implement a shell.
- Do not make the palette execute arbitrary closures against app state in the core API.
- Do not publish a stable crate from this experiment until the API is proven.

## Vocabulary

### Action

A semantic capability exposed by an application.

Examples:

```text
app.quit
config.open
theme.switch
workspace.close
git.branch.checkout
```

An action has stable identity and metadata. It does not know how it is rendered.

### Command

A user-facing presentation of an action in a context.

Examples:

```text
Palette row:   Theme: Switch
Menu item:     Switch Theme
Help row:      Ctrl-T    Switch Theme
Toolbar item:  [Theme]
```

### Invocation

A request to run an action with resolved arguments.

Example:

```text
theme.switch(theme = "catppuccin")
```

The palette should emit invocations. The application should handle them.

### Renderer

A layout-specific presentation of palette state.

Examples:

- modal renderer
- flat overlay renderer
- split preview renderer
- fullscreen renderer
- inline dropdown renderer

Renderers should draw a prepared view model. They should not own matching, selection, argument
collection, preview lifecycle, or application dispatch.

## Design Principles

### Actions are semantic

Good:

```rust
ActionSpec {
    id: ActionId::new("theme.switch"),
    title: "Switch Theme".into(),
}
```

Bad:

```rust
ActionSpec {
    x: 10,
    y: 4,
    border_style,
}
```

### The palette does not mutate application state

The palette should return events:

```rust
PaletteEvent::Invoke(invocation)
PaletteEvent::PreviewChanged(preview)
PaletteEvent::Cancel
```

The application decides what those events mean.

### Layout is replaceable

The same action list should work with bordered modal, borderless flat, split preview, fullscreen,
and embedded dropdown layouts.

### The first API should be boring

Prefer simple structs and enums over clever traits. Traits can be added once concrete use cases
prove they are needed.

### The experiment should preserve escape hatches

Applications should be able to provide their own matcher, provide their own renderer, ignore
previews, dispatch invocations however they want, and build action lists dynamically.

### Module layout should follow concepts

Do not put the experiment into one large source file. Keep files named for the concepts they own,
and prefer direct `name.rs` modules over broad helper buckets.

Good module owners:

- `id.rs` for action and input identifiers
- `input.rs` for input declarations and choice values
- `invocation.rs` for resolved arguments and invocation requests
- `spec.rs` for action metadata and availability
- `event.rs` for palette events, movement, and interaction mode
- `state.rs` for filtering, selection, input collection, previews, and invocation emission
- `view.rs` for renderable view snapshots and rows
- `render/` for Ratatui drawing code

Keep weak abstractions close to their first use. Move them outward only after they have earned an
independent concept name or multiple callers depend on the same stable boundary.

### Docs are part of the experiment

Document current behavior as it exists, not future aspirations. Roadmap material belongs in this PRD
or issues until an implementation makes it true.

When public experimental types are added, keep these surfaces aligned:

- crate README
- crate-level Rustdoc
- module Rustdoc
- examples
- Betamax tape notes for rendered behavior

Examples should prove representative use rather than only construct types. For this experiment, the
first useful example should show ownership, action dispatch, preview events, and terminal behavior
that can be captured by Betamax.

## User Experience

### Opening the palette

The application decides the keybinding. Common candidates are `Ctrl-P`, `Ctrl-Shift-P`, and `:`.
The palette appears as an overlay or embedded command surface.

### Searching

Typing filters actions:

```text
> theme
```

Example results:

```text
Theme: Switch
Theme: Preview
Theme: Export
```

The first version may use simple case-insensitive substring matching. Fuzzy matching can be
introduced behind a trait or feature once the rest of the API feels right.

### Selecting

Users can move selection with `Up`, `Down`, `PageUp`, `PageDown`, `Home`, and `End`. Relative
movement wraps at the top and bottom of the visible result set; `Home` and `End` jump to absolute
boundaries.

### Invoking

`Enter` accepts the selected row. If the action has no required arguments, the palette emits an
invocation event. If the action requires arguments, the palette moves into argument collection.

### Cancelling

`Esc` closes the palette. If a transient preview is active, cancellation should emit an event that
lets the application roll back preview state.

### Argument collection

An action may define simple inputs:

```rust
Text
Choice
Bool
```

Argument collection should happen inline inside the palette, not by opening separate dialogs.

### Preview

Preview is optional. There are two preview classes:

1. Informational preview.
1. Transient application preview.

Informational preview shows text or widgets. Transient preview emits a candidate invocation as
selection changes, and the app may temporarily apply the theme, show a side preview, or ignore the
event. The palette should not directly apply or roll back application state.

## API Hypothesis

The experiment should start with a low-level, data-oriented action model.

```rust
pub struct ActionId(String);
```

```rust
pub struct ActionSpec {
    id: ActionId,
    title: String,
    description: Option<String>,
    category: Option<String>,
    keywords: Vec<String>,
    inputs: Vec<ActionInput>,
    availability: Availability,
}
```

`ActionSpec` should expose constructors, accessors, and builder-style modifiers. The fields carry
API invariants and should not be public until the experiment proves that direct struct literals are
worth the compatibility cost.

```rust
pub enum Availability {
    Enabled,
    Disabled { reason: String },
    Hidden,
}
```

```rust
pub enum ActionInput {
    Text {
        id: InputId,
        label: String,
        placeholder: Option<String>,
    },
    Choice {
        id: InputId,
        label: String,
        choices: Vec<ActionChoice>,
    },
    Bool {
        id: InputId,
        label: String,
    },
}
```

```rust
pub struct ActionChoice {
    value: String,
    label: String,
    description: Option<String>,
}
```

`ActionChoice` should expose constructors, accessors, and builder-style modifiers. The stable
choice value becomes part of resolved invocation arguments, so callers should not depend on direct
field mutation.

```rust
pub struct ActionInvocation {
    id: ActionId,
    args: ActionArgs,
    source: InvocationSource,
}
```

`ActionInvocation` should also keep fields private. Constructing invocations through
`ActionInvocation::new` or `ActionInvocation::with_args` keeps argument and source handling
consistent across palette, keybinding, menu, mouse, and automation surfaces.

```rust
pub enum InvocationSource {
    Palette,
    KeyBinding,
    Menu,
    Mouse,
    Automation,
}
```

The palette state machine consumes action specs and emits events:

```rust
pub enum PaletteEvent {
    Invoke(ActionInvocation),
    PreviewChanged(Option<ActionInvocation>),
    Opened,
    Closed,
    Cancelled,
}
```

## Palette State

The palette should own interaction state:

```rust
pub struct PaletteState {
    query: String,
    selected: Option<usize>,
    mode: PaletteMode,
    args: ActionArgs,
}
```

Possible modes:

```rust
pub enum PaletteMode {
    Searching,
    CollectingInput { action: ActionId, input_index: usize },
}
```

## Palette View Model

Renderers should consume a prepared view model.

```rust
pub struct PaletteView {
    prompt: String,
    query: String,
    rows: Vec<PaletteRow>,
    selected: Option<usize>,
    mode: PaletteMode,
}
```

```rust
pub struct PaletteRow {
    action_id: ActionId,
    title: String,
    subtitle: Option<String>,
    category: Option<String>,
    shortcut: Option<String>,
    availability: Availability,
}
```

`PaletteView` and `PaletteRow` should expose accessors rather than public fields. Renderers should
depend on the view contract, not the current storage representation.

This separates:

```text
action model -> matching/ranking -> palette view -> renderer
```

The labs implementation currently uses owned `String` and `Vec` fields in `PaletteView`. That is an
intentional experiment-stage simplification: owned view data is easy to test, snapshot, render, and
debug while the API is still moving. Before stabilizing or publishing these APIs, revisit whether
the view should borrow data with lifetimes. Accessors already hide the storage representation so
that a future borrowed view can be evaluated without first breaking every renderer. Rendering is a
frequent operation, so a stable API should avoid unnecessary allocation once the view shape is
proven.

## Renderer Hypothesis

```rust
pub trait PaletteRenderer {
    fn render(&self, area: Rect, buf: &mut Buffer, view: &PaletteView);
}
```

The first implementation only needs one polished renderer plus enough structure to prove another
renderer could be added without changing the action model.

## Matching and Ranking

Start simple:

- case-insensitive substring match
- match against title
- match against category
- match against keywords
- stable ordering for equal matches

Potential later additions:

- fuzzy scoring
- acronym matching
- recency boost
- frequency boost
- contextual boost
- custom matcher trait

Do not add a matcher trait until the concrete implementation shows where it belongs.

## Crate and Module Shape

The current experiment uses named crates for the concept owners and small modules inside those
crates:

```text
crates/ratatui-action/src/lib.rs
crates/ratatui-action/src/id.rs
crates/ratatui-action/src/input.rs
crates/ratatui-action/src/invocation.rs
crates/ratatui-action/src/spec.rs
crates/ratatui-command-palette/src/lib.rs
crates/ratatui-command-palette/src/event.rs
crates/ratatui-command-palette/src/render/
crates/ratatui-command-palette/src/state.rs
crates/ratatui-command-palette/src/view.rs
```

Avoid broad `utils`, `common`, `helpers`, or `types` modules. File names should make ownership and
review scope obvious. `lib.rs` should stay a crate overview and module map unless the crate has a
genuinely small, coherent surface.

The experiment now uses separate workspace crates for the semantic action model and command
palette. These crates are still experimental labs crates; do not treat them as stable published APIs
until the experiment has enough evidence.

The crate ownership is:

```text
ratatui-action
  id          ActionId, InputId
  input       ActionInput, ActionChoice
  invocation  ActionArgs, ActionInvocation, InvocationSource
  spec        ActionSpec, Availability

ratatui-command-palette
  event  PaletteEvent, PaletteMode, MoveSelection
  key    normalized key commands and crossterm conversion
  matching default query filtering
  state  PaletteState
  view   PaletteView, PaletteRow
  render PaletteRenderer, built-in renderers
  shortcut presentation-only shortcut labels

ratatui-labs
  action           ratatui-action namespace
  command_palette  ratatui-command-palette namespace
```

## Example Application

The first example should prove the API with fake app actions:

```text
app.quit
help.open
theme.switch
layout.reset
debug.toggle
```

The example should demonstrate opening the palette, searching, scrolling, moving selection,
invoking an action, disabled action rendering, text input, choice input, boolean input, preview
events, and cancellation rollback for transient previews.

Current path:

```text
crates/ratatui-command-palette/examples/command-palette.rs
```

## Testing and Visual Validation

Unit tests should cover filtering, ranking, selection movement, disabled or hidden behavior,
invocation event emission, argument collection, cancel behavior, and preview event emission.

Use Betamax as the terminal-rendering validation path once the experiment changes visible TUI
behavior. Betamax should render the real example application and capture artifacts under
`target/betamax/`, including PNG, GIF, and terminal state output where useful.

Keep two tape categories:

- behavioral validation tapes that cover interaction sequences, state changes, and regressions
- rendered example tapes that showcase each built-in renderer with meaningful visible options

Use the Betamax default font size for both categories unless the tape intentionally demonstrates a
specific presentation size.

Prefer adding or updating Betamax tape steps when a change affects:

- row selection
- expansion or collapse behavior
- status and title bars
- scrolling
- colors and selection highlighting
- wrapping
- keyboard interactions
- jj-rendered template output

Betamax is better evidence than a PTY smoke test for user-visible behavior because it captures the
rendered terminal output and interaction timeline. PTY smoke tests remain useful for quick command
lifecycle checks, but they do not prove color, spacing, wrapping, row highlighting, or terminal
layout.

This experiment should also feed useful testing-tool ideas back into Betamax. Betamax is still new,
so friction with tape authoring, cwd handling, wait diagnostics, artifact inspection, alternate
screen capture, keyboard input, or state assertions should be recorded as concrete Betamax feedback
when it affects the command palette validation workflow.

When Betamax artifacts are presented to reviewers, the tape should be paced for human inspection.
Use short delays for input, longer holds for stable UI states, and checkpoint PNG/state artifacts
when the viewer needs to read details. A useful starting rhythm is 300-500 ms between typed input
and submission, 300-700 ms after simple UI transitions, 1.5-2.5 seconds for a simple stable screen,
reading-time-based holds for dense output, and 4-5 seconds on the final GIF state before the loop
restarts.

## Validation

Run:

```sh
cargo +nightly fmt --all
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
just betamax
```

Inspect generated artifacts under `target/betamax/`, especially screenshots and the GIF. If the
change is visual, include the relevant PNG or GIF path in the final report.

## Milestones

### Milestone 1: PRD only

Add this document and no implementation.

Success criteria:

- problem is clear
- action and palette split is explicit
- non-goals prevent over-frameworking
- API hypothesis is concrete enough for review

### Milestone 2: Minimal action model

Add experimental data types.

Success criteria:

- can describe actions without closures
- can describe required inputs
- can represent enabled, disabled, and hidden actions
- can construct invocations

### Milestone 3: Palette state machine

Add query, selection, filtering, and invocation events.

Success criteria:

- no rendering required to test behavior
- app dispatch remains external
- unit tests cover event behavior

### Milestone 4: First renderers and examples

Add built-in renderers and example apps.

Success criteria:

- usable in an example app
- modal, flat overlay, split preview, fullscreen, and inline dropdown renderers consume the same
  view model
- examples demonstrate switching renderers and comparing the built-in renderer set
- renderer consumes a view model
- renderer does not perform matching or dispatch
- Betamax tape captures the real example and visible interaction sequence

### Milestone 5: Argument collection, preview, and rendered interaction coverage

Add inline text, choice, and boolean input collection plus preview change events.

Success criteria:

- theme-switching example can preview values
- search example can collect text inline
- debug example can collect a boolean value inline
- cancelling can roll back previews at the app level
- renderer scrolls the selected row into view
- Betamax artifacts show scrolling, preview, cancellation, text input, boolean input, and selection
  behavior

### Milestone 6: Extraction decision

Decision: keep the extracted crate shape.

The experiment justifies keeping the focused crates:

```text
ratatui-action
ratatui-command-palette
```

The split reflects a real ownership boundary. `ratatui-labs` remains a namespaced facade for the
experiment rather than the implementation owner.

## Defaults Chosen

These defaults are intentionally conservative and can be revisited with evidence from additional
examples:

1. `ActionSpec` stores owned `String` values. This keeps the experiment easy to construct,
   snapshot, and render. Public accessors return borrowed string slices or iterators so storage can
   change later.
1. Shortcuts stay out of `ActionSpec`. They are context-specific keymap presentation, represented
   by `ShortcutLabels` when building a palette view.
1. Disabled actions are visible by default and cannot be accepted.
1. Hidden actions are omitted from ordinary search results. There is no hidden-search mode yet.
1. Choice inputs use static choices in `ActionInput::Choice`. Dynamic choices remain application
   owned until multiple examples prove a reusable shape.
1. Preview is emitted as `PaletteEvent::PreviewChanged`, not stored as an action property.
1. Matching is case-insensitive substring matching over title, category, and keywords. Fuzzy
   matching, recency, frequency, and context ranking are delegated to applications for now.
1. Renderer styling stays minimal. Built-in renderers expose titles only. More style knobs should
   wait until examples prove which options are stable concepts.
1. Crossterm integration is a small adapter: `PaletteKey::from_crossterm` converts backend key
   events into palette commands, while applications retain policy decisions such as whether `Esc`
   exits or cancels.
1. The smallest compelling example is the current command palette demo: it covers search,
   scrolling, disabled rows, text input, choice input, boolean input, preview, cancellation, and
   invocation.
