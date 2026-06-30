use ratatui::layout::Rect;
use ratatui_derive::FromRect;

#[derive(FromRect)]
#[layout(
    vertical,
    margin = 1,
    horizontal_margin = 2,
    vertical_margin = 3,
    flex = SpaceEvenly,
    spacing = 1
)]
struct AllConstraints {
    #[length(1)]
    length: Rect,

    #[min(2)]
    min: Rect,

    #[max(3)]
    max: Rect,

    #[percentage(25)]
    percentage: Rect,

    #[ratio(1, 2)]
    ratio: Rect,

    #[fill(1)]
    fill: Rect,
}

fn main() {
    let areas = AllConstraints::from(Rect::new(0, 0, 100, 20));
    let _ = (
        areas.length,
        areas.min,
        areas.max,
        areas.percentage,
        areas.ratio,
        areas.fill,
    );
}
