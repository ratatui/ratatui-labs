//! Derive macros for Ratatui.
//!
//! See [`FromRect`] for the full derive documentation, including supported attributes,
//! generated code shape, and examples.

#![allow(clippy::needless_continue)]

mod from_rect;

use proc_macro::TokenStream;
use syn::DeriveInput;

/// `FromRect` derives `From<Rect>` for structs that name the [`Rect`]s produced by a Ratatui
/// [`Layout`]. It is intended for app code that repeatedly splits the same area into named
/// regions.
///
/// ```rust
/// use ratatui::layout::Rect;
/// use ratatui_derive::FromRect;
///
/// #[derive(FromRect)]
/// #[layout(horizontal, spacing = 2)]
/// struct ButtonDemoAreas {
///     #[length(46)]
///     gallery: Rect,
///
///     #[min(34)]
///     debug: Rect,
/// }
///
/// let area = Rect::new(0, 0, 100, 20);
/// let areas = ButtonDemoAreas::from(area);
/// assert_eq!(areas.gallery.width, 46);
/// ```
///
/// The generated implementation is equivalent to writing the layout split by hand:
///
/// ```rust
/// # use ratatui::layout::Rect;
/// # struct ButtonDemoAreas {
/// #     gallery: Rect,
/// #     debug: Rect,
/// # }
/// impl From<Rect> for ButtonDemoAreas {
///     fn from(area: Rect) -> Self {
///         let constraints = [
///             ratatui::layout::Constraint::Length(46),
///             ratatui::layout::Constraint::Min(34),
///         ];
///         let [gallery, debug] = ratatui::layout::Layout::horizontal(constraints)
///             .spacing(2)
///             .areas(area);
///         Self { gallery, debug }
///     }
/// }
/// ```
///
/// # Supported input
///
/// `FromRect` supports structs with named fields. Tuple structs, unit structs, enums, and unions
/// are rejected. The derive does not check that each field is typed as [`Rect`]; Rust type
/// checking rejects incompatible field types when the generated `Self { field }` expression is
/// compiled.
///
/// Each struct needs exactly one layout direction:
///
/// ```rust
/// # use ratatui::layout::Rect;
/// # use ratatui_derive::FromRect;
/// #[derive(FromRect)]
/// #[layout(horizontal)]
/// struct HorizontalAreas {
///     #[fill(1)]
///     main: Rect,
/// }
///
/// #[derive(FromRect)]
/// #[layout(vertical)]
/// struct VerticalAreas {
///     #[fill(1)]
///     main: Rect,
/// }
/// ```
///
/// # Layout options
///
/// The `#[layout(...)]` attribute accepts these options:
///
/// - `horizontal` or `vertical`: required direction flag.
/// - `spacing = expr`: calls [`Layout::spacing`].
/// - `margin = expr`: calls [`Layout::margin`].
/// - `horizontal_margin = expr`: calls [`Layout::horizontal_margin`].
/// - `vertical_margin = expr`: calls [`Layout::vertical_margin`].
/// - `flex = Variant`: calls [`Layout::flex`] with a [`Flex`] variant.
/// - `crate = path`: uses `path::layout` instead of the default `::ratatui::layout`.
///
/// When `margin` is combined with `horizontal_margin` or `vertical_margin`, the generated code
/// applies `margin` first and then the axis-specific margins. This makes `margin` the default for
/// both axes and lets axis-specific options override one side.
///
/// The default path is `::ratatui::layout`, which is the right path for applications that depend
/// on the main `ratatui` crate. Crates that depend on `ratatui-core` directly can use
/// `#[layout(crate = ratatui_core)]`. Renamed dependencies work the same way because the value is
/// a Rust path:
///
/// ```rust
/// use ratatui as tui;
/// use ratatui_derive::FromRect;
///
/// #[derive(FromRect)]
/// #[layout(horizontal, crate = tui)]
/// struct Areas {
///     #[fill(1)]
///     main: tui::layout::Rect,
/// }
/// ```
///
/// Layout options can be combined:
///
/// ```rust
/// # use ratatui::layout::Rect;
/// # use ratatui_derive::FromRect;
/// #[derive(FromRect)]
/// #[layout(vertical, margin = 1, horizontal_margin = 2, spacing = 1)]
/// struct BodyAreas {
///     #[fill(1)]
///     main: Rect,
/// }
/// ```
///
/// # Flex variants
///
/// `flex = Variant` accepts Ratatui [`Flex`] variants by their variant name: `Start`, `End`,
/// `Center`, `SpaceBetween`, `SpaceAround`, `SpaceEvenly`, and `Legacy`.
///
/// ```rust
/// # use ratatui::layout::Rect;
/// # use ratatui_derive::FromRect;
/// #[derive(FromRect)]
/// #[layout(horizontal, flex = Center)]
/// struct CenterAreas {
///     #[length(1)]
///     main: Rect,
/// }
/// ```
///
/// # Field constraints
///
/// Each field needs exactly one constraint attribute. The field order is the layout order, and the
/// field names become the names of the generated [`Rect`]s.
///
/// ```rust
/// # use ratatui::layout::Rect;
/// # use ratatui_derive::FromRect;
/// #[derive(FromRect)]
/// #[layout(vertical)]
/// struct EveryConstraint {
///     #[length(1)]
///     length: Rect,
///
///     #[min(2)]
///     min: Rect,
///
///     #[max(3)]
///     max: Rect,
///
///     #[percentage(25)]
///     percentage: Rect,
///
///     #[ratio(1, 2)]
///     ratio: Rect,
///
///     #[fill(1)]
///     fill: Rect,
/// }
/// ```
///
/// The constraint values are Rust expressions passed to the matching [`Constraint`] variant. The
/// derive does not evaluate them:
///
/// ```rust
/// # use ratatui::layout::Rect;
/// # use ratatui_derive::FromRect;
/// const SIDEBAR_WIDTH: u16 = 30;
/// const NUMERATOR: u32 = 1;
/// const DENOMINATOR: u32 = 3;
///
/// #[derive(FromRect)]
/// #[layout(
///     horizontal,
///     spacing = SIDEBAR_WIDTH / 30,
///     margin = 1 + 1,
///     horizontal_margin = SIDEBAR_WIDTH / 10,
///     vertical_margin = 2,
/// )]
/// struct ExpressionAreas {
///     #[length(SIDEBAR_WIDTH + 2)]
///     sidebar: Rect,
///
///     #[ratio(NUMERATOR, DENOMINATOR)]
///     content: Rect,
///
///     #[fill(1 + 1)]
///     extra: Rect,
/// }
/// ```
///
/// `FromRect` intentionally supports only the six field attributes shown above. It does not
/// support wrapper attributes or raw constraint expressions.
///
/// [`Constraint`]: https://docs.rs/ratatui/latest/ratatui/layout/enum.Constraint.html
/// [`Flex`]: https://docs.rs/ratatui/latest/ratatui/layout/enum.Flex.html
/// [`Layout`]: https://docs.rs/ratatui/latest/ratatui/layout/struct.Layout.html
/// [`Layout::flex`]: https://docs.rs/ratatui/latest/ratatui/layout/struct.Layout.html#method.flex
/// [`Layout::horizontal_margin`]: https://docs.rs/ratatui/latest/ratatui/layout/struct.Layout.html#method.horizontal_margin
/// [`Layout::margin`]: https://docs.rs/ratatui/latest/ratatui/layout/struct.Layout.html#method.margin
/// [`Layout::spacing`]: https://docs.rs/ratatui/latest/ratatui/layout/struct.Layout.html#method.spacing
/// [`Layout::vertical_margin`]: https://docs.rs/ratatui/latest/ratatui/layout/struct.Layout.html#method.vertical_margin
/// [`Rect`]: https://docs.rs/ratatui/latest/ratatui/layout/struct.Rect.html
#[proc_macro_derive(
    FromRect,
    attributes(layout, length, min, max, percentage, ratio, fill)
)]
pub fn derive_from_rect(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    from_rect::expand(&input).into()
}
