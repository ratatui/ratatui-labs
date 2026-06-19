# 0001 Command Palette Crate Shape

## Status

Accepted.

## Context

The command palette experiment has two separable concepts:

- semantic actions, inputs, availability, arguments, and invocation requests
- command palette state, filtering, selection, rendering, previews, and examples

Keeping both concepts in `ratatui-labs` made the first spike quick, but it also blurred ownership.
The API surface was already large enough that a single file made review harder and encouraged
unrelated changes to land together.

The Rust API Guidelines emphasize future proofing, type safety, and construction APIs for complex
values. The Microsoft Pragmatic Rust Guidelines also favor smaller crates when a boundary is real
and names can stay concise.

## Decision

Split the experiment into concept-owned crates:

- `ratatui-action` owns semantic action types.
- `ratatui-command-palette` owns palette state, events, view data, renderers, and examples.
- `ratatui-labs` remains an umbrella crate with namespaced access to the focused crates.

Keep modules named after the concept they own. Avoid broad `types`, `utils`, `common`, or
`helpers` modules until a stable shared concept earns that name. Keep crate root files as overview
and module maps rather than broad implementation files.

The current module ownership is:

```text
ratatui-action
  id          action and input identifiers
  input       input declarations and choices
  invocation  resolved arguments and invocation requests
  spec        action metadata and availability

ratatui-command-palette
  event   palette events, movement, and interaction mode
  key  normalized key commands and crossterm conversion
  matching  default query filtering
  state   filtering, selection, input collection, previews, and invocation emission
  view    renderable view snapshots and rows
  render  Ratatui renderers
  shortcut  presentation-only shortcut labels

ratatui-labs
  action           namespaced ratatui-action access
  command_palette  namespaced ratatui-command-palette access
```

## Consequences

This makes dependency direction clear: the palette depends on actions, while actions do not depend
on Ratatui rendering or terminal concerns. It also lets the action model be tested independently
from one UI surface.

The cost is workspace overhead. There are more manifests, READMEs, docs, and package boundaries to
keep aligned. That cost is acceptable because the split reflects a real domain boundary and avoids
making `ratatui-labs` a grab bag.

## Alternatives Considered

### Keep Everything In `ratatui-labs`

This is lowest overhead for a short-lived spike. It becomes worse as soon as the action model is
used by keymaps, menus, help screens, or automation, because the palette crate would no longer be
the natural owner.

### Put Everything In `ratatui-command-palette`

This makes the first visible feature easy to package, but it incorrectly makes the action model feel
palette-specific. The action model is meant to be reusable across multiple invocation surfaces.

### Create More Crates Immediately

Separate crates for matching, renderer traits, or examples would be premature. Those boundaries are
not yet proven by multiple callers.

## Revisit When

- another action surface needs the action model, such as keybindings, menus, or help
- the palette needs a matcher or renderer extension point with multiple implementations
- the umbrella facade creates confusing docs or dependency behavior
