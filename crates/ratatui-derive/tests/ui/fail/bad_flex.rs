use ratatui::layout::Rect;
use ratatui_derive::FromRect;

#[derive(FromRect)]
#[layout(horizontal, flex = Stretch)]
struct Areas {
    #[fill(1)]
    main: Rect,
}

fn main() {}
