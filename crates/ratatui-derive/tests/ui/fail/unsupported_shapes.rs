use ratatui::layout::Rect;
use ratatui_derive::FromRect;

#[derive(FromRect)]
#[layout(horizontal)]
struct Tuple(#[fill(1)] Rect);

#[derive(FromRect)]
#[layout(horizontal)]
struct Unit;

#[derive(FromRect)]
#[layout(horizontal)]
enum Enum {
    Area,
}

#[derive(FromRect)]
#[layout(horizontal)]
union Union {
    area: Rect,
}

fn main() {}
