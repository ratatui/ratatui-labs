use ratatui::layout::Rect;
use ratatui_derive::FromRect;

#[derive(FromRect)]
#[layout(horizontal)]
struct Areas {
    #[length(1)]
    #[min(1)]
    main: Rect,
}

fn main() {}
