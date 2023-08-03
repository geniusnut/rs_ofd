use raqote::{DrawOptions, DrawTarget, PathBuilder, Point, SolidSource, Source, Transform};

use font_kit::family_name::FamilyName;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;

fn main() {
    let mut draw_target = DrawTarget::new(400, 400);

    draw_target.clear(SolidSource {
        r: 0xFF,
        g: 0xFF,
        b: 0xFF,
        a: 0xFF,
    });

    let black = Source::Solid(SolidSource {
        r: 0x0,
        g: 0x0,
        b: 0x0,
        a: 0xFF,
    });

    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();

    draw_target.draw_text(
        &font,
        24.,
        "Hello!",
        Point::new(100., 100.),
        &black,
        &DrawOptions::new(),
    );

    let transform = Transform::rotation(Angle::);

    draw_target.set_transform(&transform);

    draw_target.draw_text(
        &font,
        24.,
        "Hello!",
        Point::new(100., 200.),
        &black,
        &DrawOptions::new(),
    );

    let transform = Transform::rotation(Angle::radians(-0.1));

    draw_target.set_transform(&transform);

    draw_target.draw_text(
        &font,
        24.,
        "Hello!",
        Point::new(100., 300.),
        &black,
        &DrawOptions::new(),
    );

    draw_target.write_png("test-raqote.png").unwrap();
}