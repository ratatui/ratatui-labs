# ratatui-derive

Derive macros and code generation helpers for Ratatui APIs.

This crate is experimental and lives in
[`ratatui/ratatui-labs`](https://github.com/ratatui/ratatui-labs). It is published with a beta
version while the macro names, attribute grammar, and generated public contracts are evaluated in
real Ratatui applications.

The crate currently provides `FromRect`, a derive macro for named layout-area structs. It
generates `impl From<Rect> for YourStruct`, so application code can split a `Rect` into ordinary
named fields without a local layout macro.

```rust
use ratatui::layout::Rect;
use ratatui_derive::FromRect;

#[derive(FromRect)]
#[layout(horizontal, spacing = 2)]
struct ButtonDemoAreas {
    #[length(46)]
    gallery: Rect,

    #[min(34)]
    debug: Rect,
}

let area = Rect::new(0, 0, 100, 20);
let areas = ButtonDemoAreas::from(area);
```

This keeps the layout result as regular Rust:

- `ButtonDemoAreas` is a named type.
- `gallery` and `debug` are ordinary fields.
- `From<Rect>` works with `ButtonDemoAreas::from(area)` and `area.into()`.
- The generated split can be tested through normal Rust tests.

## Supported Shape

`FromRect` supports structs with named fields. Each field must have one layout constraint
attribute:

```rust
#[length(10)]
#[min(20)]
#[max(30)]
#[percentage(50)]
#[ratio(1, 3)]
#[fill(1)]
```

The struct must choose one direction with `#[layout(horizontal)]` or `#[layout(vertical)]`.
Layout options include spacing, margin, axis-specific margins, flex, and crate path override:

```rust
use ratatui::layout::Rect;
use ratatui_derive::FromRect;

#[derive(FromRect)]
#[layout(vertical, margin = 1, horizontal_margin = 2, flex = Center)]
struct Areas {
    #[fill(1)]
    main: Rect,
}
```

By default, generated code uses `::ratatui::layout`. Lower-level crates that depend on
`ratatui-core` directly can use `#[layout(crate = ratatui_core)]`. Renamed dependencies work the
same way:

```rust
use ratatui as tui;
use ratatui_derive::FromRect;

#[derive(FromRect)]
#[layout(horizontal, crate = tui)]
struct Areas {
    #[fill(1)]
    main: tui::layout::Rect,
}
```

See the [`FromRect` docs](https://docs.rs/ratatui-derive/latest/ratatui_derive/derive.FromRect.html)
for the complete generated code shape, supported attributes, and compile-time error behavior.

## Status

`ratatui-derive` is separate from the main `ratatui` crate and is not re-exported by `ratatui`.
Add it as a direct dependency when using the derive macros. The API may change before a stable
release.
