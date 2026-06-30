use ratatui as tui;
use ratatui_derive::FromRect;

#[derive(FromRect)]
#[layout(horizontal, crate = tui)]
struct AliasAreas {
    #[fill(1)]
    main: tui::layout::Rect,
}

fn main() {
    let areas = AliasAreas::from(tui::layout::Rect::new(0, 0, 10, 10));
    let _ = areas.main;
}
