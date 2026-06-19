# Ratatui Command Palette PRD

## Status

Experimental.

This document describes a command palette experiment for `ratatui-labs`. It is not a commitment to
a published crate, a stable API, or inclusion in Ratatui core.

The goal is to test whether Ratatui applications benefit from a reusable action model plus command
palette presentation, and to identify the smallest API that could later justify focused crates such
as `ratatui-action` and `ratatui-command-palette`.

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

Good early module owners:

- `action.rs` for action identity, metadata, availability, and invocation types
- `command_palette.rs` for palette state, events, filtering, and selection behavior
- `command_palette/view.rs` only after the renderer needs a separate view model owner
- `command_palette/render.rs` only after rendering exists

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

Users can move selection with `Up`, `Down`, `PageUp`, `PageDown`, `Home`, and `End`.

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
    pub id: ActionId,
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub keywords: Vec<String>,
    pub inputs: Vec<ActionInput>,
    pub availability: Availability,
}
```

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
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}
```

```rust
pub struct ActionInvocation {
    pub id: ActionId,
    pub args: ActionArgs,
    pub source: InvocationSource,
}
```

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
pub struct PaletteView<'a> {
    pub query: &'a str,
    pub rows: &'a [PaletteRow],
    pub selected: Option<usize>,
    pub preview: Option<&'a PalettePreview>,
    pub mode: PaletteModeView<'a>,
}
```

```rust
pub struct PaletteRow {
    pub action_id: ActionId,
    pub title: String,
    pub subtitle: Option<String>,
    pub category: Option<String>,
    pub shortcut: Option<String>,
    pub availability: Availability,
}
```

This separates:

```text
action model -> matching/ranking -> palette view -> renderer
```

## Renderer Hypothesis

```rust
pub trait PaletteRenderer {
    fn render(&self, area: Rect, buf: &mut Buffer, view: PaletteView<'_>);
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

During the experiment, start with named modules inside the existing labs crate:

```text
crates/ratatui-labs/src/action.rs
crates/ratatui-labs/src/command_palette.rs
```

Split into directories only when the concept has enough internal structure to justify it:

```text
crates/ratatui-labs/src/action/
crates/ratatui-labs/src/command_palette/
```

Avoid broad `utils`, `common`, `helpers`, or `types` modules. File names should make ownership and
review scope obvious.

Do not create a published `ratatui-action` or `ratatui-command-palette` crate until the experiment
has enough evidence.

The future extraction target, if successful, is likely:

```text
ratatui-action
  ActionId
  ActionSpec
  ActionInput
  ActionInvocation
  Availability

ratatui-command-palette
  PaletteState
  PaletteEvent
  PaletteView
  PaletteRenderer
  built-in renderers
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

The example should demonstrate opening the palette, searching, moving selection, invoking an action,
disabled action rendering, argument collection for theme switching, and optional preview events for
theme switching.

Suggested path:

```text
examples/command-palette.rs
```

or:

```text
examples/command-palette/
```

depending on repository conventions.

## Testing and Visual Validation

Unit tests should cover filtering, ranking, selection movement, disabled or hidden behavior,
invocation event emission, argument collection, cancel behavior, and preview event emission.

Use Betamax as the terminal-rendering validation path once the experiment changes visible TUI
behavior. Betamax should render the real example application and capture artifacts under
`target/betamax/`, including PNG, GIF, and terminal state output where useful.

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

### Milestone 4: First renderer and example

Add a modal renderer and an example app.

Success criteria:

- usable in an example app
- renderer consumes a view model
- renderer does not perform matching or dispatch
- Betamax tape captures the real example and visible interaction sequence

### Milestone 5: Argument collection and preview

Add choice input collection and preview change events.

Success criteria:

- theme-switching example can preview values
- cancelling can roll back previews at the app level
- Betamax artifacts show preview, cancellation, and selection behavior

### Milestone 6: Extraction decision

Decide whether the experiment justifies:

```text
ratatui-action
ratatui-command-palette
```

or should remain in labs.

## Open Questions

1. Should `ActionSpec` use `String`, `Cow<'static, str>`, or Ratatui `Line<'static>`?
1. Should shortcuts live in `ActionSpec`, or in a separate keymap model?
1. Should disabled actions be visible by default in the palette?
1. Should hidden actions be searchable with an explicit mode?
1. Should choice inputs support dynamic choices?
1. Should preview be a property of an action or only an event emitted by palette state?
1. Should fuzzy matching be built in, optional, or delegated?
1. How much styling should renderers expose?
1. Should recent or frequent command ranking exist in labs, or be left to applications?
1. What is the smallest compelling example app?
