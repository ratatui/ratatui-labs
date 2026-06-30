use ratatui::layout::Rect;
use ratatui_derive::FromRect;

#[derive(FromRect)]
#[layout(horizontal, flex = Start)]
struct StartAreas {
    #[length(1)]
    main: Rect,
}

#[derive(FromRect)]
#[layout(horizontal, flex = End)]
struct EndAreas {
    #[length(1)]
    main: Rect,
}

#[derive(FromRect)]
#[layout(horizontal, flex = Center)]
struct CenterAreas {
    #[length(1)]
    main: Rect,
}

#[derive(FromRect)]
#[layout(horizontal, flex = SpaceBetween)]
struct SpaceBetweenAreas {
    #[length(1)]
    first: Rect,

    #[length(1)]
    second: Rect,
}

#[derive(FromRect)]
#[layout(horizontal, flex = SpaceAround)]
struct SpaceAroundAreas {
    #[length(1)]
    first: Rect,

    #[length(1)]
    second: Rect,
}

#[derive(FromRect)]
#[layout(horizontal, flex = SpaceEvenly)]
struct SpaceEvenlyAreas {
    #[length(1)]
    first: Rect,

    #[length(1)]
    second: Rect,
}

#[derive(FromRect)]
#[layout(horizontal, flex = Legacy)]
struct LegacyAreas {
    #[length(1)]
    main: Rect,
}

fn main() {
    let area = Rect::new(0, 0, 10, 10);
    let _ = StartAreas::from(area).main;
    let _ = EndAreas::from(area).main;
    let _ = CenterAreas::from(area).main;
    let space_between = SpaceBetweenAreas::from(area);
    let _ = (space_between.first, space_between.second);
    let space_around = SpaceAroundAreas::from(area);
    let _ = (space_around.first, space_around.second);
    let space_evenly = SpaceEvenlyAreas::from(area);
    let _ = (space_evenly.first, space_evenly.second);
    let _ = LegacyAreas::from(area).main;
}
