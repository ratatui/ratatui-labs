use ratatui::layout::Rect;
use ratatui_derive::FromRect;

#[derive(FromRect)]
#[layout(horizontal)]
struct Areas {
    main: Rect,
}

fn main() {}
