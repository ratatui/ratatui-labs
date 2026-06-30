use ratatui::layout::Rect;
use ratatui_derive::FromRect;

const WIDTH: u16 = 10;
const GAP: i32 = 2;
const NUMERATOR: u32 = 1;
const DENOMINATOR: u32 = 3;

fn fill_weight() -> u16 {
    2
}

#[derive(FromRect)]
#[layout(
    horizontal,
    spacing = GAP,
    margin = WIDTH / 10,
    horizontal_margin = WIDTH / 5,
    vertical_margin = WIDTH / 2
)]
struct ExpressionValues {
    #[length(WIDTH + 1)]
    length: Rect,

    #[min(WIDTH - 2)]
    min: Rect,

    #[max(WIDTH + 10)]
    max: Rect,

    #[percentage(WIDTH * 5)]
    percentage: Rect,

    #[ratio(NUMERATOR, DENOMINATOR)]
    ratio: Rect,

    #[fill(fill_weight())]
    fill: Rect,
}

fn main() {
    let areas = ExpressionValues::from(Rect::new(0, 0, 100, 20));
    let _ = (
        areas.length,
        areas.min,
        areas.max,
        areas.percentage,
        areas.ratio,
        areas.fill,
    );
}
