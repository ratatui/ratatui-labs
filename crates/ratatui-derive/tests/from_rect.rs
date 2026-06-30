use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui_derive::FromRect;

#[derive(Debug, Eq, PartialEq, FromRect)]
#[layout(horizontal, spacing = 2)]
struct ButtonDemoAreas {
    #[length(46)]
    gallery: Rect,

    #[min(34)]
    debug: Rect,
}

#[derive(Debug, Eq, PartialEq, FromRect)]
#[layout(vertical, margin = 1, flex = Center)]
struct VerticalAreas {
    #[length(1)]
    header: Rect,

    #[fill(1)]
    body: Rect,
}

#[derive(Debug, Eq, PartialEq, FromRect)]
#[layout(vertical, margin = 1, horizontal_margin = 2, vertical_margin = 3)]
struct AxisMarginAreas {
    #[fill(1)]
    main: Rect,
}

#[derive(Debug, Eq, PartialEq, FromRect)]
#[layout(horizontal)]
struct InherentFromAreas {
    #[fill(1)]
    main: Rect,
}

impl InherentFromAreas {
    const fn from(_area: Rect) -> Self {
        Self {
            main: Rect::new(9, 9, 9, 9),
        }
    }
}

#[test]
fn from_rect_matches_direct_horizontal_layout() {
    let area = Rect::new(0, 0, 100, 20);
    let areas = ButtonDemoAreas::from(area);
    let [gallery, debug] = Layout::horizontal([Constraint::Length(46), Constraint::Min(34)])
        .spacing(2)
        .areas(area);

    assert_eq!(areas, ButtonDemoAreas { gallery, debug });
}

#[test]
fn from_rect_matches_direct_vertical_layout() {
    let area = Rect::new(0, 0, 100, 20);
    let areas = VerticalAreas::from(area);
    let [header, body] = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)])
        .margin(1)
        .flex(Flex::Center)
        .areas(area);

    assert_eq!(areas, VerticalAreas { header, body });
}

#[test]
fn from_rect_matches_direct_layout_with_axis_margins() {
    let area = Rect::new(0, 0, 100, 20);
    let areas = AxisMarginAreas::from(area);
    let [main] = Layout::vertical([Constraint::Fill(1)])
        .margin(1)
        .horizontal_margin(2)
        .vertical_margin(3)
        .areas(area);

    assert_eq!(areas, AxisMarginAreas { main });
}

#[test]
fn inherent_from_can_coexist_with_generated_from_impl() {
    let area = Rect::new(0, 0, 100, 20);
    let inherent = InherentFromAreas::from(area);
    let trait_from = <InherentFromAreas as From<Rect>>::from(area);
    let into: InherentFromAreas = area.into();
    let [main] = Layout::horizontal([Constraint::Fill(1)]).areas(area);

    assert_eq!(inherent.main, Rect::new(9, 9, 9, 9));
    assert_eq!(trait_from, InherentFromAreas { main });
    assert_eq!(into, InherentFromAreas { main });
}

#[test]
fn trybuild_passes() {
    let test_cases = trybuild::TestCases::new();
    test_cases.pass("tests/ui/pass/*.rs");
}

#[test]
fn trybuild_failures() {
    let test_cases = trybuild::TestCases::new();
    test_cases.compile_fail("tests/ui/fail/*.rs");
}
