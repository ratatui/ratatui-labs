//! Wraps styled Ratatui text before it is rendered.
//!
//! [`WrapAlgorithm::FirstFit`] and [`WrapAlgorithm::OptimalFit`] adapt textwrap's low-level
//! line-breaking algorithms to styled Ratatui graphemes. [`WrapAlgorithm::ParagraphCompat`] is a
//! separate implementation that reproduces Ratatui Paragraph's reflow behavior.
//!
//! [`TextWrapper`] converts strings, spans, lines, and other values accepted by [`Text`] into an
//! owned `Text<'static>` at a requested terminal width. The owned result can be cached until its
//! source, width, or [`WrapAlgorithm`] changes, then rendered by an ordinary [`Paragraph`].
//!
//! # Quick start
//!
//! ```
//! use ratatui::style::Stylize;
//! use ratatui::text::Line;
//! use ratatui::widgets::Paragraph;
//! use ratatui_textwrap::{TextWrapper, WrapAlgorithm};
//!
//! let line = Line::from(vec!["Styled ".blue(), "text can wrap across spans".bold()]);
//! let wrapping = TextWrapper::new().algorithm(WrapAlgorithm::OptimalFit);
//! let wrapped = wrapping.wrap(line, 20);
//! let paragraph = Paragraph::new(wrapped);
//! ```
//!
//! `wrapped` exists as a separate value because wrapping allocates owned strings and line
//! collections. Passing the width to [`TextWrapper::wrap`] lets the same configuration serve a
//! resizable view.
//!
//! Custom widgets and text-processing pipelines can wrap one [`Line`] at a time through the
//! [`algorithms`] module. Its functions expose owned line output without exposing textwrap
//! fragments or Paragraph's internal queues.
//!
//! # Choosing an algorithm
//!
//! - [`WrapAlgorithm::FirstFit`] is the default. It greedily fills each line.
//! - [`WrapAlgorithm::OptimalFit`] considers the complete source line and minimizes textwrap's
//!   default monospace penalty cost. It may choose a small overflow when that costs less than an
//!   unbalanced break.
//! - [`WrapAlgorithm::ParagraphCompat`] runs the copied Ratatui Paragraph reflow state machine.
//!   Choose it when matching that historical whitespace and long-word behavior matters more than
//!   using textwrap's fragment model.
//!
//! First-fit and optimal-fit omit separator whitespace after the final fragment on a generated
//! line. `ParagraphCompat` accepts the same `trim` policy as `ratatui::widgets::Wrap` and preserves
//! Paragraph's distinct decisions at whitespace boundaries.
//!
//! # Text contract
//!
//! Wrapping preserves [`Text`] style and alignment. Each generated line copies the style and
//! alignment of its source [`Line`]. Span styles survive word splitting, including a word composed
//! from differently styled spans. Adjacent output spans with equal styles may be coalesced;
//! rendered cells are preserved, but the original span partition is not.
//!
//! Each entry in [`Text::lines`] is a hard boundary. Newline and tab control characters embedded
//! directly in a [`Span`] are filtered by Ratatui's styled-grapheme iterator; they do not create
//! more lines. With first-fit or optimal-fit, a single grapheme wider than the requested width
//! remains intact on an overflowing line; Paragraph compatibility omits it to match Paragraph. A
//! width of zero returns no lines and retains the top-level style and alignment.
//!
//! # textwrap integration
//!
//! The textwrap algorithms receive fragments prepared from Ratatui's styled graphemes:
//!
//! 1. [`CellWidth`] measures terminal cells, and Ratatui classifies each grapheme as word content
//!    or whitespace.
//! 1. Graphemes are grouped into [`Fragment`] values containing a word and its following
//!    whitespace. A style boundary inside a word is not a break point.
//! 1. Overlong words are split at Ratatui grapheme boundaries before line breaking because
//!    textwrap's low-level algorithms arrange complete fragments.
//! 1. The selected fragment slices are rebuilt as owned Ratatui lines.
//!
//! The adapter calls textwrap's low-level [`wrap_first_fit`] and [`wrap_optimal_fit`] functions. It
//! does not use textwrap's high-level wrapping, width calculation, word separator, or word
//! splitter. See the [`Fragment` source], [first-fit source], and [optimal-fit source] for the
//! upstream contracts used here.
//!
//! # Experimental status
//!
//! `ratatui-textwrap` is an experiment, and its public API may change while the configuration and
//! compatibility surface are evaluated. The owned output and algorithm behavior described above
//! are the current contract. The crate does not replace [`Paragraph::wrap`] or provide lazy layout,
//! source-position mapping, cursor mapping, indentation, hyphenation, or configurable optimal-fit
//! penalties.
//!
//! The [design notes] explain the decisions behind the current API, the differences found while
//! comparing it with Paragraph, and possible follow-up work. Those directions are evaluation
//! topics, not commitments in the current release.
//!
//! [`CellWidth`]: ratatui_core::buffer::CellWidth
//! [`Fragment`]: https://docs.rs/textwrap/0.16/textwrap/core/trait.Fragment.html
//! [`Fragment` source]: https://github.com/mgeisler/textwrap/blob/4770e55af425a0cffb9ad8496599d2a1a4f5ed14/src/core.rs#L217-L230
//! [`Line`]: ratatui_core::text::Line
//! [`Paragraph`]: https://docs.rs/ratatui/0.30.2/ratatui/widgets/struct.Paragraph.html
//! [`Paragraph::wrap`]: https://docs.rs/ratatui/0.30.2/ratatui/widgets/struct.Paragraph.html#method.wrap
//! [`Span`]: ratatui_core::text::Span
//! [`Text`]: ratatui_core::text::Text
//! [`Text::lines`]: ratatui_core::text::Text::lines
//! [`wrap_first_fit`]: https://docs.rs/textwrap/0.16/textwrap/wrap_algorithms/fn.wrap_first_fit.html
//! [`wrap_optimal_fit`]: https://docs.rs/textwrap/0.16/textwrap/wrap_algorithms/fn.wrap_optimal_fit.html
//! [first-fit source]: https://github.com/mgeisler/textwrap/blob/4770e55af425a0cffb9ad8496599d2a1a4f5ed14/src/wrap_algorithms.rs#L336-L367
//! [optimal-fit source]: https://github.com/mgeisler/textwrap/blob/4770e55af425a0cffb9ad8496599d2a1a4f5ed14/src/wrap_algorithms/optimal_fit.rs#L302-L381
//! [design notes]: https://github.com/ratatui/ratatui-labs/blob/main/crates/ratatui-textwrap/docs/design.md
#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]
#![warn(rustdoc::bare_urls)]
#![warn(rustdoc::broken_intra_doc_links)]
#![warn(rustdoc::redundant_explicit_links)]

/// Line-level wrapping algorithms for custom widgets and text-processing pipelines.
pub mod algorithms;
/// Public wrapping configuration and orchestration.
mod wrapper;

/// Selects the line-breaking strategy.
#[doc(inline)]
pub use algorithms::WrapAlgorithm;
/// Configures owned styled-text wrapping.
#[doc(inline)]
pub use wrapper::TextWrapper;
