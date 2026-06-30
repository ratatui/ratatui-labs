use ratatui::layout::Rect;
use ratatui_derive::FromRect;

#[derive(FromRect)]
struct Areas {
    #[fill(1)]
    main: Rect,
}

fn main() {}
