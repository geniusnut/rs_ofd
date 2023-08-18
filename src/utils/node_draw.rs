#![allow(dead_code)]

use std::fmt::{Display, Formatter};
use lazy_static::lazy_static;
use raqote::*;
use crate::ofd::{PhysicalBox};
use xmltree::Element;
use std::sync::Mutex;
use std::collections::HashMap;
use font_kit::family_name::FamilyName;
use font_kit::properties::{Properties, Weight};
use font_kit::source::SystemSource;
use font_kit::loaders::core_text::Font;
use image::{RgbaImage};

const SONGTI_LIST: &[&str] = &["Songti", "STSong", "SimSong", "FangSong", "Songti SC",];
const KAITI_LIST: &[&str] = &["KaiTi", "Kai"];

lazy_static! {
    pub static ref MUTEX_JBIG_RES: Mutex<HashMap<String, RgbaImage>> = {
        let m = HashMap::new();
        Mutex::new(m)
    };

    pub static ref MUTEX_IMAGE_RES: Mutex<HashMap<String, String>> = {
        let m = HashMap::new();
        Mutex::new(m)
    };

    pub static ref MUTEX_FONT_RES: Mutex<HashMap<String, Font>> = {
        let m = HashMap::new();
        Mutex::new(m)
    };

    pub static ref FONT_NAME_2_FONT_MAP: Mutex<HashMap<String, Font>> = {
        let m = HashMap::new();
        Mutex::new(m)
    };

    pub static ref FONT_FAMILY_NAME_MAP: HashMap<String, &'static [&'static str]> = {
        let mut m = HashMap::new();
        m.insert("宋体".to_string(), SONGTI_LIST);
        m.insert("Kai".to_string(), KAITI_LIST);
        m.insert("楷体".to_string(), KAITI_LIST);
        m.insert("KaiTi".to_string(), KAITI_LIST);
        m
    };
}

pub fn get_font_from_family_name(family_name: &str) -> Font {
    let k = FONT_FAMILY_NAME_MAP.get(family_name).map_or(
        vec![FamilyName::Title(String::from(family_name))],
        |cs|cs.iter().map(|e| FamilyName::Title(String::from(*e))).collect()
    );
    // println!("family_name: {}, candidates: {:?}", family_name, k);
    if FONT_NAME_2_FONT_MAP.lock().unwrap().get(family_name).is_none() {
        let t = SystemSource::new().select_best_match(
            &k,
            &Properties::new().weight(Weight::NORMAL),
        ).unwrap()
            .load()
            .unwrap();
        FONT_NAME_2_FONT_MAP.lock().unwrap().insert(family_name.to_string(), t);
    }
    return FONT_NAME_2_FONT_MAP.lock().unwrap().get(family_name).unwrap().clone();
}

const PATH_OBJECT: &'static str = "PathObject";
const TEXT_OBJECT: &'static str = "TextObject";
const IMAGE_OBJECT: &'static str = "ImageObject";
pub const PPMM: f32 = 7.559;  // pixel per mm, ppi = 192; 25.4mm = 1inch

// const DRAW_OBJECT: Vec<&str> = vec![PATH_OBJECT, TEXT_OBJECT,  IMAGE_OBJECT];

struct OfdColor {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Default for OfdColor {
    fn default() -> Self {
        OfdColor {
            r: 0, g: 0, b: 0, a: 0xff,
        }
    }
}
impl OfdColor {
    fn solid_source(&self) -> Source {
        Source::Solid(SolidSource {
            r: self.r,
            g: self.g,
            b: self.b,
            a: self.a,
        })
    }
}

fn ofd_color_from_v(s: &String) -> OfdColor {
    let mut iter = s.split_whitespace().into_iter();
    OfdColor {
        r: iter.next().unwrap().parse().unwrap(),
        g: iter.next().unwrap().parse().unwrap(),
        b: iter.next().unwrap().parse().unwrap(),
        a: iter.next().unwrap_or("255").parse().unwrap(),
    }
}

macro_rules! unwrap_or_return {
    ( $e:expr ) => {
        match $e {
            Some(x) => x,
            None => return,
        }
    }
}

/// TODO: using stack to optimize recursion, 'cause the compiler's tail-recursion is not guaranteed
///
pub fn draw_ele_recursive(dt: &mut DrawTarget, element: &Element) {
    for child in &element.children {
        let ele = unwrap_or_return!(child.as_element());
        match ele.name.as_str() {
            PATH_OBJECT => {
                draw_path(dt, ele)
            },
            TEXT_OBJECT => {
                draw_text(dt, ele)
            },
            IMAGE_OBJECT => {
                draw_image(dt, ele)
            },
            _ => {
                draw_ele_recursive(dt, ele);
            },
        }
    }
}

pub fn draw_path(dt: &mut DrawTarget, element: &Element) {
    let line_width: f32 = element.attributes.get("LineWidth").unwrap_or(&String::from("0.25"))
                            .parse().unwrap();
    let boundary = element.attributes.get("Boundary")
        .map(|b| boundary_to_rect(b)).unwrap_or(PhysicalBox::default());
    // println!("draw_path line_width: {}, boundary: {:?}", line_width, boundary);
    let fill_color = element.get_child("FillColor").map_or(OfdColor::default(),
                                                           |e| ofd_color_from_v(e.attributes.get("Value").unwrap()));
    let stroke_color = element.get_child("StrokeColor").map_or(OfdColor::default(),
                                                               |e| ofd_color_from_v(e.attributes.get("Value").unwrap()));
    let abbr_data = element.get_child("AbbreviatedData").unwrap().get_text().unwrap().clone();
    println!("abbr_data: {}", abbr_data);
    let tokens= abbreviate_data(&String::from(abbr_data.as_ref()));
    let trans = dt.get_transform().clone();
    let t = trans.pre_translate(Vector::new(boundary.x, boundary.y)).clone();
    // println!("   {:?}, {:?}", trans, t);
    dt.set_transform(&t);
    draw_abbreviate_path(dt, &Point::new(0., 0.), tokens, &line_width, &fill_color, &stroke_color);
    dt.set_transform(&trans);
}

pub fn boundary_to_rect(boundary: &String) -> PhysicalBox {
    let mut iter = boundary.split_whitespace().into_iter();
    PhysicalBox {
        x: iter.next().unwrap().parse().unwrap(),
        y: iter.next().unwrap().parse().unwrap(),
        width: iter.next().unwrap().parse().unwrap(),
        height: iter.next().unwrap().parse().unwrap(),
    }
}

fn attr_to_transform(ctm: &String) -> Transform {
    let vec: Vec<f32> = ctm.split_whitespace().map(|s| s.parse().unwrap()).collect();

    Transform::new(
        vec[0], vec[1], vec[2], vec[3], vec[4], vec[5],
    )
}
/// Drawing TextObject
pub fn draw_text(dt: &mut DrawTarget, element: &Element) {
    let boundary = element.attributes.get("Boundary")
        .map(|b| boundary_to_rect(b)).unwrap_or(PhysicalBox::default());

    let font_id = element.attributes.get("Font").unwrap();
    let size: f32 = element.attributes.get("Size").map_or(1.0, |x| x.parse().unwrap());
    let fill_color = element.get_child("FillColor").map_or(OfdColor::default(),
                       |e| ofd_color_from_v(e.attributes.get("Value").unwrap()));
    // let stroke_color = element.get_child("StrokeColor");
    let text_code = element.get_child("TextCode").unwrap();
    let mut start_p = Point::new(text_code.attributes.get("X").map_or(0., |x| x.parse().unwrap()),
                             text_code.attributes.get("Y").map_or(0., |y| y.parse().unwrap()));

    let ctm = element.attributes.get("CTM").map_or(Transform::identity(), |x|
        attr_to_transform(x));
    // let font = MUTEX_FONT_RES.lock().unwrap().get(font_id).unwrap_or(
    //     &SystemSource::new().select_best_match(
    //         &[FamilyName::Title("KaiTi".into())],
    //         &Properties::new().weight(Weight::NORMAL),
    //     ).unwrap()
    //    .load()
    //    .unwrap()
    // ).clone();

    let font = MUTEX_FONT_RES.lock().unwrap().get(font_id).unwrap().clone();

    let mut ids = Vec::new();
    let mut positions = Vec::new();
    let text = text_code.get_text().unwrap().clone();

    let mut iter_delta_x = text_code.attributes.get("DeltaX").map_or(vec![0.; text.len()].into_iter(), |s| delta_to_vec(s).into_iter());
    let mut iter_delta_y = text_code.attributes.get("DeltaY").map_or(vec![0.; text.len()].into_iter(), |s| delta_to_vec(s).into_iter());

    start_p.x += boundary.x;
    start_p.y += boundary.y;

    let m = dt.get_transform().clone();

    // the ctm is implied on the original point [boundary.left_top]
    let mm = Transform::identity().pre_translate(Vector::new(-boundary.x,-boundary.y))
        .then(&ctm)
        .then_translate(Vector::new(boundary.x, boundary.y))
        .then(&m);
    dt.set_transform(&mm);
    // start_p = m.transform_point(start_p);
    for c in text.chars() {
        let id = font.glyph_for_char(c).unwrap();
        ids.push(id);
        positions.push(Point::new(start_p.x, start_p.y));
        // let offset_p = m.transform_point(Point::new(iter_delta_x.next().unwrap_or(0.),  iter_delta_y.next().unwrap_or(0.)));
        let offset_p = Point::new(iter_delta_x.next().unwrap_or(0.),  iter_delta_y.next().unwrap_or(0.));
        start_p.x += offset_p.x;
        start_p.y += offset_p.y;
    }
    let options = &DrawOptions::new();
    let point_size = size; // * PPMM
    // FIXME: TRANSFORM NOT APPLY TO POINT_SIZE
    dt.draw_glyphs(&font, point_size, &ids, &positions, &fill_color.solid_source(), options);
    dt.set_transform(&m);
}

fn draw_image(dt: &mut DrawTarget, element: &Element) {
    let _id = element.attributes.get("ID").unwrap();
    let resource_id = element.attributes.get("ResourceID").unwrap();
    let boundary = element.attributes.get("Boundary");
    let img_file = MUTEX_IMAGE_RES.lock().unwrap().get(resource_id).unwrap().clone();

    // println!("draw_image ID: {}, resource_id: {}, img_file: {}", id, resource_id, img_file);
    let img = MUTEX_JBIG_RES.lock().unwrap().get(&img_file).unwrap().clone();

    let physical_box = boundary_to_rect(boundary.unwrap());
    let d1: Vec<u32> = img.pixels().into_iter().map(
        |p| ((p[3] as u32) << 24) | ((p[2] as u32) << 16) | ((p[1] as u32) << 8) | (p[0] as u32))
    .collect();

    let image = Image {
        width: img.width() as i32,
        height: img.height() as i32,
        data: &d1,
    };
    // println!("image size: {}*{}, {}; physical_box: {:?}", image.width, image.height,
    //          image.data.len(), physical_box);
    dt.draw_image_with_size_at(physical_box.width, physical_box.height, physical_box.x, physical_box.y, &image, &DrawOptions::new());
}

union _PathToken {
    op: char,
    v: f32,
}

enum Tag { C, F }

struct PathToken {
    tag: Tag,
    token: _PathToken,
}


impl Display for PathToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unsafe {
            match self {
                PathToken { tag: Tag::C, token: _PathToken { op: _} } => write!(f, "{}", self.token.op),
                PathToken { tag: Tag::F, token: _PathToken { v: _} } => write!(f, "{}", self.token.v),
            }
        }
    }
}

fn draw_abbreviate_path(dt: &mut DrawTarget, start_p: &Point, path: Vec<PathToken>, line_width: &f32, 
    #[allow(unused_variables)] fill_color: &OfdColor, stroke_color: &OfdColor) {
    let mut idx = 0;
    let mut pb = PathBuilder::new();
    while idx < path.len() {
        unsafe {
            match path.get(idx) {
                Some(PathToken{ tag: Tag::C, token: _PathToken {op: 'M'}}) => {
                    idx += 1;
                    let x = path.get(idx).unwrap().token.v;
                    idx += 1;
                    let y = path.get(idx).unwrap().token.v;
                    pb.move_to(x, y);
                },
                Some(PathToken{ tag: Tag::C, token: _PathToken { op: 'L'}}) => {
                    let mut iter = path[idx+1..idx+3].iter().map(|pt|pt.token.v);
                    idx += 2;
                    let x = iter.next().unwrap();
                    let y = iter.next().unwrap();
                    pb.line_to(x, y);
                },
                Some(PathToken{ tag: Tag::C, token: _PathToken { op: 'B'}}) => {
                    let mut iter = path[idx+1..idx+7].iter().map(|pt|pt.token.v);
                    idx += 6;
                    pb.cubic_to(iter.next().unwrap(), iter.next().unwrap(),
                                iter.next().unwrap(), iter.next().unwrap(),
                                iter.next().unwrap(), iter.next().unwrap(),);
                },
                Some(PathToken{ tag: Tag::C, token: _PathToken { op: 'A'}}) => {
                    // pb.arc()
                },
                Some(PathToken{ tag: Tag::C, token: _PathToken { op: 'Q'}}) => {
                    let mut iter = path[idx + 1..idx + 5].iter().map(|pt|pt.token.v);
                    idx += 4;

                    pb.quad_to(iter.next().unwrap(), iter.next().unwrap(),
                                iter.next().unwrap(), iter.next().unwrap(),);
                },
                Some(PathToken{ tag: Tag::C, token: _PathToken { op: 'C'}}) => {
                },
                Some(pt @ PathToken{ tag: Tag::C, token: _}) => {
                    panic!("OFD path_token [{}] invalid!", pt);
                },
                Some(_) => {},
                None => {},
            }
        }
        idx += 1;
    }
    pb.close();
    let path = pb.finish();
    let new_path = path.transform(&Transform::translation(start_p.x, start_p.y));
    dt.stroke(&new_path, &Source::Solid(SolidSource {
        r: stroke_color.r,
        g: stroke_color.g,
        b: stroke_color.b,
        a: stroke_color.a,
    }), &StrokeStyle {
        width: line_width.clone(),
        ..Default::default()
    }, &DrawOptions::new());
}

fn abbreviate_data(data: &String) -> Vec<PathToken> {
    data.split_whitespace().map(|s| match s {
        "M"|"L"|"B"|"A"|"Q"|"C" => PathToken{ tag: Tag::C, token: _PathToken { op: s.chars().next().unwrap() }},
        v => PathToken { tag: Tag::F, token: _PathToken {v: v.parse().unwrap()}},
    }).collect()
}

fn delta_to_vec(data: &String) -> Vec<f32> {
    let mut vec: Vec<f32> = Vec::new();
    let mut iter = data.split_whitespace().into_iter();
    while let Some(e) = iter.next() {
        match e {
            "g" => {
                let c: usize = iter.next().unwrap().parse().unwrap();
                let v: f32 = iter.next().unwrap().parse().unwrap();
                vec.extend(vec![v; c]);
            },
            v => vec.push(v.parse().unwrap())
        }
    }
    vec
}

#[cfg(test)]
mod tests {
    use font_kit::family_name::FamilyName;
    use font_kit::properties::{Properties, Weight};
    use font_kit::source::SystemSource;
    use raqote::*;
    use crate::utils::node_draw::{FONT_FAMILY_NAME_MAP, get_font_from_family_name};
    use super::{delta_to_vec, OfdColor};
    use super::{abbreviate_data, draw_abbreviate_path, };
    #[test]
    fn test_dt_matrix() {
        let mut dt = DrawTarget::new(400, 400);
        dt.fill_rect(0., 0., 400., 400., &Source::Solid( SolidSource{r: 0xff,
            g: 0xff,
            b: 0xff,
            a: 0xff,}), &DrawOptions::new());
        let p_size = 1.0;
        dt.fill_rect(200.-p_size, 200.-p_size, p_size*2., p_size*2., &Source::Solid( SolidSource{
            r: 0x00,
            g: 0x00,
            b: 0x00,
            a: 0xff,}), &DrawOptions::new());

        let font = get_font_from_family_name("Kaiti");
        let boundary = PhysicalBox {
            x: 200.0,
            y: 200.0,
            width: 70.0,
            height: 40.0,
        };
        //  [0.323729, -0.944243, 0.9342, 0.327214, 0.0, 0.0], text: 全
        let v = vec![0.323729, -0.944243, 0.9342, 0.327214, 0.0, 0.0];
        let ctm = Transform::new(
            v[0], v[1], v[2], v[3], v[4], v[5],
        );
        let mm = Transform::identity()
            .pre_translate(Vector::new(-boundary.x, -boundary.y))
            .then(&ctm)
            .then_translate(Vector::new(boundary.x, boundary.y))
            ;
        dt.fill_rect(boundary.x, boundary.y, boundary.width, boundary.height, &Source::Solid( SolidSource{
            r: 0x34,
            g: 0x98,
            b: 0xb2,
            a: 0xff,}), &DrawOptions::new());
        dt.set_transform(&mm);
        dt.fill_rect(boundary.x, boundary.y, boundary.width, boundary.height, &Source::Solid( SolidSource{r: 0xff,
            g: 0x0,
            b: 0xff,
            a: 0xff,}), &DrawOptions::new());
        let point_size = 3.;
        let ids = vec![font.glyph_for_char('全').unwrap()];
        let positions = vec![Point::new(200., 200.); 0];
        let options = DrawOptions::new();
        // dt.set_transform(&Transform::identity());

        dt.draw_glyphs(&font, point_size, &ids, &positions, &Source::Solid( SolidSource{
            r: 0x34,
            g: 0x98,
            b: 0xb2,
            a: 0xff,}), &options);
        dt.draw_text(
            &font,
            24.,
            "全",
            Point::new(200., 200.),
            &Source::Solid(SolidSource {
                r: 0,
                g: 0,
                b: 0xff,
                a: 0xff,
            }),
            &DrawOptions::new(),
        );
        dt.write_png("test.png").unwrap();
    }

    #[test]
    fn test_delta_to_vec() {
        let d = String::from("g 11 3.175 g 3 1.5875 3.175 g 3 1.5875 3.175 g 10 1.5875");
        let v = delta_to_vec(&d);
        print!("v: {:?}", v);
    }

    #[test]
    fn test_abbrev() {
        let data = "M 10.07 5.54 B 10.07 3.04 8.04 1 5.53 1 B 3.03 1 1 3.04 1 5.54 B 1 8.04 3.03 10.08 5.53 10.08 B 8.04 10.08 10.07 8.04 10.07 5.54 M 2.3 2.3 L 8.7 8.7 M 2.3 8.7 L 8.7 2.3";
        // let data = "M 0 0 L 100 0 L 100 100 L 0 100 L 0 0";
        println!("data: {}", data);
        let tokens = abbreviate_data(&data.into());
        let mut dt = DrawTarget::new(400, 400);

        dt.fill_rect(0., 0., 400., 400., &Source::Solid( SolidSource{r: 0xff,
            g: 0xff,
            b: 0xff,
            a: 0xff,}), &DrawOptions::new());

        let line_width: f32 = 0.5;
        let fill_color = OfdColor { r: 156, g: 82, b: 32, a: 255 };
        let stroke_color = OfdColor {  r: 156, g: 82, b: 32, a: 255 };
        draw_abbreviate_path(&mut dt, &Point::new(0., 0.), tokens, &line_width, &fill_color, &stroke_color);
        dt.write_png("test_abbrev.png").expect("");
    }

    const WHITE_SOURCE: Source = Source::Solid(SolidSource {
        r: 0xff,
        g: 0xff,
        b: 0xff,
        a: 0xff,
    });

    #[test]
    fn test_raqote_path() {
        let mut dt = DrawTarget::new(400, 400);
        dt.fill_rect(0., 0., 400., 400., &Source::Solid( SolidSource{r: 0xff,
            g: 0xff,
            b: 0xff,
            a: 0xff,}), &DrawOptions::new());

        let mut pb = PathBuilder::new();

        pb.move_to(0., 0.);
        pb.line_to(100., 0.);
        pb.line_to(100., 100.);
        pb.line_to(0., 100.);
        pb.close();
        dt.set_transform(&Transform::translation(60., 60.));
        dt.stroke(&pb.finish(), &OfdColor::default().solid_source(), &StrokeStyle {
            width: 1.,
            ..Default::default()
        }, &DrawOptions::new());
        let t = dt.get_transform().pre_scale(5., 5.);
        dt.set_transform(&t);
        let mut pb = PathBuilder::new();

        pb.move_to(0., 0.);
        pb.line_to(100., 0.);
        pb.line_to(100., 100.);
        pb.line_to(0., 100.);
        pb.close();
        dt.stroke(&pb.finish(), &OfdColor::default().solid_source(), &StrokeStyle {
            width: 1.,
            ..Default::default()
        }, &DrawOptions::new());

        let font = SystemSource::new()
            .select_best_match(&[FamilyName::SansSerif], &Properties::new())
            .unwrap()
            .load()
            .unwrap();

        dt.draw_text(
            &font,
            24.,
            "Hello",
            Point::new(0., 100.),
            &Source::Solid(SolidSource {
                r: 0,
                g: 0,
                b: 0xff,
                a: 0xff,
            }),
            &DrawOptions::new(),
        );
        dt.write_png("test_abbrev.png").expect("");
    }

    #[test]
    fn test_sys_font() {
        // let sys_fonts = SystemSource::new().all_fonts();
        // println!("sys_fonts: {:?}", sys_fonts);
        let font = get_font_from_family_name("STSong");
        println!("font: {:?}", font.family_name());
    }

    #[test]
    fn test_select_by_postscript() {
        let font = SystemSource::new().select_by_postscript_name("KaiTi")
            .unwrap()
            .load()
            .unwrap();
        println!("font: {:?}", font);
    }

    #[test]
    fn test_map_vec() {
        let v1:Vec<u8> = vec![1, 2, 3];
        let mut v2:Vec<u32> = v1.into_iter().map(Into::<u32>::into).collect();

        // for i in 0..3 {
        //     v2[i] = v1[i] as u32;
        // }
        println!("{:?}", v2.len());
    }
}