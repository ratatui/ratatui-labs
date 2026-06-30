use ratatui_core::layout::Rect;
use ratatui_derive::FromRect;

#[derive(FromRect)]
#[layout(vertical, crate = ratatui_core)]
struct CoreAreas {
    #[length(1)]
    header: Rect,

    #[fill(1)]
    body: Rect,
}

fn main() {
    let areas = CoreAreas::from(Rect::new(0, 0, 10, 10));
    let _ = (areas.header, areas.body);
}
