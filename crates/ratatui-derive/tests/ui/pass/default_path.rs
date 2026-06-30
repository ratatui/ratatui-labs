use ratatui::layout::Rect;
use ratatui_derive::FromRect;

#[derive(FromRect)]
#[layout(horizontal, spacing = 2)]
struct ButtonDemoAreas {
    #[length(46)]
    gallery: Rect,

    #[min(34)]
    debug: Rect,
}

fn main() {
    let areas = ButtonDemoAreas::from(Rect::new(0, 0, 100, 20));
    let _ = (areas.gallery, areas.debug);
}
