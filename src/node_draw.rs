#![allow(dead_code)]

use crate::ofd::{ImageObject, PathObject, PhysicalBox, TextObject, DrawParam, _Color, Appearance};
use font_kit::family_name::FamilyName;
use font_kit::font::Font;
use font_kit::properties::{Properties, Weight};
use font_kit::source::SystemSource;
use image::RgbaImage;
use lazy_static::lazy_static;
use raqote::*;
use send_wrapper::SendWrapper;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Mutex;
use xmltree::Element;
use crate::backends::DrawBackend;
use crate::backends::DrawError::OutputError;

const SONGTI_LIST: &[&str] = &["Songti", "STSong", "SimSong", "FangSong", "Songti SC"];
const KAITI_LIST: &[&str] = &["KaiTi", "Kai"];

lazy_static! {
    pub static ref MUTEX_RGB_IMAGE_RES: Mutex<HashMap<String, RgbaImage>> = {
        let m = HashMap::new();
        Mutex::new(m)
    };
    pub static ref MUTEX_IMAGE_RES: Mutex<HashMap<String, String>> = {
        let m = HashMap::new();
        Mutex::new(m)
    };
    pub static ref MUTEX_RES_DRAW_PARAMS: Mutex<HashMap<String, DrawParam>> = {
        let m = HashMap::new();
        Mutex::new(m)
    };

    pub static ref RES_FONT_ID_MAP: Mutex<HashMap<String, SendWrapper<Font>>> = {
        let m = HashMap::new();
        Mutex::new(m)
    };
    pub static ref FONT_NAME_2_FONT_MAP: Mutex<HashMap<String, SendWrapper<Font>>> = {
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
        |cs| {
            cs.iter()
                .map(|e| FamilyName::Title(String::from(*e)))
                .collect()
        },
    );
    println!("family_name: {}, candidates: {:?}", family_name, k);
    if FONT_NAME_2_FONT_MAP
        .lock()
        .unwrap()
        .get(family_name)
        .is_none()
    {
        let t = SystemSource::new()
            .select_best_match(&k, &Properties::new().weight(Weight::NORMAL))
            .unwrap()
            .load()
            .unwrap();
        FONT_NAME_2_FONT_MAP
            .lock()
            .unwrap()
            .insert(family_name.to_string(), SendWrapper::new(t));
    }
    return FONT_NAME_2_FONT_MAP
        .lock()
        .unwrap()
        .get(family_name)
        .unwrap()
        .clone()
        .take();
}

const PATH_OBJECT: &'static str = "PathObject";
const TEXT_OBJECT: &'static str = "TextObject";
const IMAGE_OBJECT: &'static str = "ImageObject";
pub const PPMM: f32 = 7.559; // pixel per mm, ppi = 192; 25.4mm = 1inch

// const DRAW_OBJECT: Vec<&str> = vec![PATH_OBJECT, TEXT_OBJECT,  IMAGE_OBJECT];

#[derive(Debug)]
struct OfdColor {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Default for OfdColor {
    fn default() -> Self {
        OfdColor {
            r: 0,
            g: 0,
            b: 0,
            a: 0xff,
        }
    }
}

impl OfdColor {
    fn alpha_source(&self) -> Source {
        Source::Solid(SolidSource {
            r: self.r,
            g: self.g,
            b: self.b,
            a: 0x30,
        })
    }

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

macro_rules! unwrap_or_continue {
    ( $e:expr ) => {
        match $e {
            Some(x) => x,
            None => continue,
        }
    };
}

pub struct RaqoteDrawBackend {
    pub dt: DrawTarget,
}

impl RaqoteDrawBackend {
    pub fn new(width: i32, height: i32) -> RaqoteDrawBackend {
        let mut dt = DrawTarget::new(width, height);
        dt.fill_rect(
            0.,
            0.,
            width as f32,
            height as f32,
            &Source::Solid(SolidSource {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            }),
            &DrawOptions::new(),
        );
        dt.set_transform(&Transform::scale(PPMM, PPMM));
        RaqoteDrawBackend {
            dt
        }
    }
}

impl DrawBackend for RaqoteDrawBackend {
    fn output_page(&mut self, out_f_name: &String) -> crate::backends::Result<()> {
        println!("output page: {}", out_f_name);
        self.dt.write_png(out_f_name).map_err(|e| {
            OutputError(format!("write png file {} failed: {}", out_f_name, e))
        })
    }

    fn draw_boundary(&mut self, boundary: &PhysicalBox) {
        println!("draw boundary: {:#?}", boundary);
        let trans = self.dt.get_transform().clone();
        let t = Transform::identity()
            .then_translate(Vector::new(boundary.x, boundary.y))
            .then(&trans)
            .clone();
        // println!("   {:?}, {:?}", trans, t);
        self.dt.set_transform(&t);
    }

    fn save(&mut self) -> crate::backends::Transform {
        crate::backends::Transform::from(self.dt.get_transform().clone())
    }

    fn restore(&mut self, transform: &crate::backends::Transform) {
        self.dt.set_transform(&Transform::new(
            transform.m11,
            transform.m12,
            transform.m21,
            transform.m22,
            transform.m31,
            transform.m32,
        ));
    }

    fn draw_path_object(&mut self, draw_param_id: Option<&String>, path_object: &PathObject) {
        draw_path_object(&mut self.dt, draw_param_id, path_object);
    }

    fn draw_text_object(&mut self, draw_param_id: Option<&String>, text_object: &TextObject) {
        draw_text_object(&mut self.dt, draw_param_id, text_object);
    }

    fn draw_image_object(&mut self, image_object: &ImageObject) {
        draw_image_object(&mut self.dt, image_object);
    }
}

/// TODO: using stack to optimize recursion, 'cause the compiler's tail-recursion is not guaranteed
///
pub fn draw_ele_recursive(backend: &mut RaqoteDrawBackend, element: &Element) {
    for child in &element.children {
        let ele = unwrap_or_continue!(child.as_element());
        match ele.name.as_str() {
            PATH_OBJECT => draw_path(&mut backend.dt, ele),
            TEXT_OBJECT => draw_text(&mut backend.dt, ele),
            IMAGE_OBJECT => draw_image(&mut backend.dt, ele),
            _ => {
                draw_ele_recursive(backend, ele);
            }
        }
    }
}

pub fn draw_path_object(dt: &mut DrawTarget, draw_param_id: Option<&String>, path_object: &PathObject) {
    let draw_param = draw_param_id.map_or(
        None,
        |it| {
            Some(MUTEX_RES_DRAW_PARAMS.lock().unwrap().get(it).unwrap().clone())
        }
    );
    let line_width: f32 = path_object.line_width.unwrap_or(0.5);
    let boundary = &path_object.boundary;

    let tokens = abbreviate_data(&path_object.abbreviated_data);
    let ctm = path_object.ctm.clone().map_or(
        Transform::identity(),
        |s| attr_to_transform(&s)
    );

    let fill_color = path_object.fill_color.clone().map_or(
        OfdColor::default(),
        |s| match s.value {
            None => draw_param.as_ref().map_or(
                OfdColor::default(),
                |dp| dp.fill_color.clone().map_or(
                    OfdColor::default(),
                    |c| ofd_color_from_v(&c.value.unwrap_or("128 0 0".to_string()))
                )
            ),
            Some(c) => ofd_color_from_v(&c),
        }
    );
    let stroke_color = path_object.stroke_color.clone().map_or(
        draw_param.as_ref().map_or(
            OfdColor::default(),
            |dp| dp.stroke_color.clone().map_or(
                OfdColor::default(),
                |c| ofd_color_from_v(&c.value.unwrap_or("128 0 0".to_string()))
            )
        ),
        |s| match s.value {
            None => draw_param.as_ref().map_or(
                OfdColor::default(),
                |dp| dp.stroke_color.clone().map_or(
                    OfdColor::default(),
                    |c| ofd_color_from_v(&c.value.unwrap_or("128 0 0".to_string()))
                )
            ),
            Some(c) => ofd_color_from_v(&c),
        }
    );
    let trans = dt.get_transform().clone();
    let t = ctm
        .then_translate(Vector::new(boundary.x, boundary.y))
        .then(&trans)
        .clone();
    // println!("   {:?}, {:?}", trans, t);
    dt.set_transform(&t);
    // println!("draw_path_object line_width: {}", line_width);
    draw_abbreviate_path(
        dt,
        &Point::new(0., 0.),
        tokens,
        &line_width,
        &fill_color,
        &stroke_color,
    );
    dt.set_transform(&trans);
}

pub fn draw_path(dt: &mut DrawTarget, element: &Element) {
    let line_width: f32 = element
        .attributes
        .get("LineWidth")
        .unwrap_or(&String::from("0.25"))
        .parse()
        .unwrap();
    let boundary = element
        .attributes
        .get("Boundary")
        .map(|b| boundary_to_rect(b))
        .unwrap_or(PhysicalBox::default());
    // println!("draw_path line_width: {}, boundary: {:?}", line_width, boundary);
    let fill_color = element
        .get_child("FillColor")
        .map_or(OfdColor::default(), |e| {
            e.attributes
                .get("Value")
                .map_or(OfdColor::default(), |v| ofd_color_from_v(v))
        });
    let stroke_color = element
        .get_child("StrokeColor")
        .map_or(OfdColor::default(), |e| {
            e.attributes
                .get("Value")
                .map_or(OfdColor::default(), |v| ofd_color_from_v(v))
            // ofd_color_from_v(e.attributes.get("Value").unwrap())
        });
    let abbr_data = element
        .get_child("AbbreviatedData")
        .unwrap()
        .get_text()
        .unwrap()
        .clone();
    // println!("abbr_data: {}", abbr_data);
    let tokens = abbreviate_data(&String::from(abbr_data.as_ref()));
    let ctm = element
        .attributes
        .get("CTM")
        .map_or(Transform::identity(), |x| attr_to_transform(x));

    let trans = dt.get_transform().clone();
    let t = ctm
        .then_translate(Vector::new(boundary.x, boundary.y))
        .then(&trans)
        .clone();
    // println!("   {:?}, {:?}", trans, t);
    dt.set_transform(&t);
    draw_abbreviate_path(
        dt,
        &Point::new(0., 0.),
        tokens,
        &line_width,
        &fill_color,
        &stroke_color,
    );
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
        // vec[0], vec[1], vec[2], vec[3], vec[4], vec[5],
        vec[0], -vec[1], -vec[2], vec[3], vec[4], vec[5],
    )
}

pub fn draw_text_object(dt: &mut DrawTarget, draw_param_id: Option<&String>, text_object: &TextObject) {
    let draw_param = draw_param_id.map_or(
        None,
        |it| {
            Some(MUTEX_RES_DRAW_PARAMS.lock().unwrap().get(it).unwrap().clone())
        }
    );
    let dp_fill_color = draw_param.as_ref().map_or(
        None,
        |dp| dp.fill_color.clone().map_or(
            None,
            |c| Some(ofd_color_from_v(&c.value.unwrap_or("0 0 0".to_string())))
        )
    );
    let dp_stroke_color = draw_param.as_ref().map_or(
        None,
        |dp| dp.stroke_color.clone().map_or(
            None,
            |c| Some(ofd_color_from_v(&c.value.unwrap_or("0 0 0".to_string())))
        )
    );
    let boundary = text_object.boundary;
    let font_id = text_object.font.clone();
    let size: f32 = text_object.size;
    let fill_color = text_object.fill_color.clone().map_or(
        OfdColor::default(),
        |s| match s.value {
            None => dp_fill_color.unwrap_or(OfdColor::default()),
            Some(c) => ofd_color_from_v(&c)
        }
    );
    let font = RES_FONT_ID_MAP.lock().unwrap().get(font_id.as_str()).unwrap().clone().take();

    let ctm = text_object.ctm.clone().map_or(
        Transform::identity(),
            |s| attr_to_transform(&s));

    let text_code = text_object.text_code.clone();
    let mut iter_delta_x = text_code.delta_x.map_or(
        vec![0.; text_code.text.len()].into_iter(),
        |s| delta_to_vec(&s).into_iter()
    );

    let mut iter_delta_y = text_code.delta_y.map_or(
        vec![0.; text_code.text.len()].into_iter(),
        |s| delta_to_vec(&s).into_iter()
    );

    let mut start_p = Point::new(text_code.x, text_code.y);
    start_p.x += boundary.x;
    start_p.y += boundary.y;
    let mut positions = Vec::new();
    let point_size = size; // * PPMM

    let m = dt.get_transform().clone();
    if !ctm.eq(&Transform::identity()) {
        // WARNING: It's very wired when the ctm is not identity the x is -x in windows.
        start_p.x = -start_p.x + 2. * boundary.x;

        // println!("draw_text ctm: {:?}, text: {}", ctm, text);
        // the ctm is implied on the original point [boundary.left_top]
        let mm = Transform::identity()
            .pre_translate(Vector::new(-start_p.x, -start_p.y))
            .then(&ctm)
            .then_translate(Vector::new(start_p.x, start_p.y))
            .then(&m);
        dt.set_transform(&mm);
    } else {
        // println!("draw_text ctm: {:?}, text: {}", ctm, text);
        // the ctm is implied on the original point [boundary.left_top]
        let mm = Transform::identity()
            .pre_translate(Vector::new(-boundary.x, -boundary.y))
            .then(&ctm)
            .then_translate(Vector::new(boundary.x, boundary.y))
            .then(&m);
        dt.set_transform(&mm);
    }

    let mut ids = Vec::new();
    for c in text_code.text.chars() {
        let id = font.glyph_for_char(c).unwrap();
        ids.push(id);
        positions.push(Point::new(start_p.x, start_p.y));
        // let offset_p = m.transform_point(Point::new(iter_delta_x.next().unwrap_or(0.),  iter_delta_y.next().unwrap_or(0.)));
        let offset_p = Point::new(
            iter_delta_x.next().unwrap_or(0.),
            iter_delta_y.next().unwrap_or(0.),
        );
        start_p.x += offset_p.x;
        start_p.y += offset_p.y;
    }

    let options = &DrawOptions::new();
    dt.draw_glyphs(
        &font,
        point_size,
        &ids,
        &positions,
        &fill_color.solid_source(),
        options,
    );
    dt.set_transform(&m);
}

/// Drawing TextObject
pub fn draw_text(dt: &mut DrawTarget, element: &Element) {
    let boundary = element
        .attributes
        .get("Boundary")
        .map(|b| boundary_to_rect(b))
        .unwrap_or(PhysicalBox::default());
    let font_id = element.attributes.get("Font").unwrap();
    let size: f32 = element
        .attributes
        .get("Size")
        .map_or(1.0, |x| x.parse().unwrap());
    let fill_color = element
        .get_child("FillColor")
        .map_or(OfdColor::default(), |e| {
            e.attributes
                .get("Value")
                .map_or(OfdColor::default(), |v| ofd_color_from_v(v))
            // ofd_color_from_v(e.attributes.get("Value").unwrap())
        });
    // let stroke_color = element.get_child("StrokeColor");
    let text_code = element.get_child("TextCode").unwrap();
    let mut start_p = Point::new(
        text_code
            .attributes
            .get("X")
            .map_or(0., |x| x.parse().unwrap()),
        text_code
            .attributes
            .get("Y")
            .map_or(0., |y| y.parse().unwrap()),
    );

    let ctm = element
        .attributes
        .get("CTM")
        .map_or(Transform::identity(), |x| attr_to_transform(x));
    // let font = RES_FONT_ID_MAP.lock().unwrap().get(font_id).unwrap_or(
    //     &SystemSource::new().select_best_match(
    //         &[FamilyName::Title("KaiTi".into())],
    //         &Properties::new().weight(Weight::NORMAL),
    //     ).unwrap()
    //    .load()
    //    .unwrap()
    // ).clone();

    let font = RES_FONT_ID_MAP
        .lock()
        .unwrap()
        .get(font_id)
        .unwrap()
        .clone()
        .take();
    let mut ids = Vec::new();
    let mut positions = Vec::new();
    let text = text_code.get_text().unwrap().clone();

    let mut iter_delta_x = text_code
        .attributes
        .get("DeltaX")
        .map_or(vec![0.; text.len()].into_iter(), |s| {
            delta_to_vec(s).into_iter()
        });
    let mut iter_delta_y = text_code
        .attributes
        .get("DeltaY")
        .map_or(vec![0.; text.len()].into_iter(), |s| {
            delta_to_vec(s).into_iter()
        });

    start_p.x += boundary.x;
    start_p.y += boundary.y;

    let m = dt.get_transform().clone();

    if !ctm.eq(&Transform::identity()) {
        // WARNING: It's very wired when the ctm is not identity the x is -x in windows.
        start_p.x = -start_p.x + 2. * boundary.x;

        // println!("draw_text ctm: {:?}, text: {}", ctm, text);
        // the ctm is implied on the original point [boundary.left_top]
        let mm = Transform::identity()
            .pre_translate(Vector::new(-start_p.x, -start_p.y))
            .then(&ctm)
            .then_translate(Vector::new(start_p.x, start_p.y))
            .then(&m);
        dt.set_transform(&mm);
    } else {
        // println!("draw_text ctm: {:?}, text: {}", ctm, text);
        // the ctm is implied on the original point [boundary.left_top]
        let mm = Transform::identity()
            .pre_translate(Vector::new(-boundary.x, -boundary.y))
            .then(&ctm)
            .then_translate(Vector::new(boundary.x, boundary.y))
            .then(&m);
        dt.set_transform(&mm);
    }

    let options = &DrawOptions::new();
    let point_size = size; // * PPMM
                           // start_p = m.transform_point(start_p);
    for c in text.chars() {
        let id = font.glyph_for_char(c).unwrap();
        ids.push(id);
        positions.push(Point::new(start_p.x, start_p.y));
        // let offset_p = m.transform_point(Point::new(iter_delta_x.next().unwrap_or(0.),  iter_delta_y.next().unwrap_or(0.)));
        let offset_p = Point::new(
            iter_delta_x.next().unwrap_or(0.),
            iter_delta_y.next().unwrap_or(0.),
        );
        start_p.x += offset_p.x;
        start_p.y += offset_p.y;
    }
    // FIXME: TRANSFORM NOT APPLY TO POINT_SIZE
    // println!("draw_text {:?}  positions: {:?}, boundary: {:?}", text.chars(), positions[0], boundary);
    // dt.set_transform(&Transform::identity()
    //                      .then_translate(Vector::new(-boundary.x, -boundary.y))
    //                      .then(&ctm)
    //                      .then_translate(Vector::new(boundary.x, boundary.y))
    //     .then_translate(Vector::new(-start_p.x, start_p.y))
    //                      .then(&m)
    // );
    dt.draw_glyphs(
        &font,
        point_size,
        &ids,
        &positions,
        &fill_color.solid_source(),
        options,
    );
    dt.set_transform(&m);
}

pub fn draw_image_object(dt: &mut DrawTarget, image_object: &ImageObject) {
    println!("draw_image_object: {:#?}", image_object);
    let _id  = image_object.id.clone();
    let img_file = MUTEX_IMAGE_RES.lock().unwrap().get(image_object.resource_id.as_str()).unwrap().clone();
    let img = MUTEX_RGB_IMAGE_RES.lock().unwrap().get(&img_file).unwrap().clone();

    let d1: Vec<u32> = img.pixels().into_iter()
        .map(|p| {
            // ((p[3] as u32) << 24) | ((p[2] as u32) << 16) | ((p[1] as u32) << 8) | (p[0] as u32)
            if p == &image::Rgba([255, 255, 255, 0]) {
                return 0;
            }
            ((p[3] as u32) << 24) | ((p[0] as u32) << 16) | ((p[1] as u32) << 8) | (p[2] as u32)
        })
        .collect();

    let image = Image {
        width: img.width() as i32,
        height: img.height() as i32,
        data: &d1,
    };
    let physical_box = image_object.boundary.clone();

    println!("draw_image_object: {:?}", physical_box);
    dt.draw_image_with_size_at(
        physical_box.width,
        physical_box.height,
        physical_box.x,
        physical_box.y,
        &image,
        &DrawOptions::new(),
    );
}

fn draw_image(dt: &mut DrawTarget, element: &Element) {
    let _id = element.attributes.get("ID").unwrap();
    let resource_id = element.attributes.get("ResourceID").unwrap();
    let boundary = element.attributes.get("Boundary");
    let img_file = MUTEX_IMAGE_RES
        .lock()
        .unwrap()
        .get(resource_id)
        .unwrap()
        .clone();

    // println!("draw_image ID: {}, resource_id: {}, img_file: {}", id, resource_id, img_file);
    let img = MUTEX_RGB_IMAGE_RES
        .lock()
        .unwrap()
        .get(&img_file)
        .expect(format!("image {} not found", img_file).as_str())
        .clone();

    let physical_box = boundary_to_rect(boundary.unwrap());
    let d1: Vec<u32> = img
        .pixels()
        .into_iter()
        .map(|p| {
            ((p[3] as u32) << 24) | ((p[0] as u32) << 16) | ((p[1] as u32) << 8) | (p[2] as u32)
        })
        .collect();

    let image = Image {
        width: img.width() as i32,
        height: img.height() as i32,
        data: &d1,
    };
    // println!("image size: {}*{}, {}; physical_box: {:?}", image.width, image.height,
    //          image.data.len(), physical_box);
    dt.draw_image_with_size_at(
        physical_box.width,
        physical_box.height,
        physical_box.x,
        physical_box.y,
        &image,
        &DrawOptions::new(),
    );
}

union _PathToken {
    op: char,
    v: f32,
}

enum Tag {
    C,
    F,
}

struct PathToken {
    tag: Tag,
    token: _PathToken,
}

impl Display for PathToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unsafe {
            match self {
                PathToken {
                    tag: Tag::C,
                    token: _PathToken { op: _ },
                } => write!(f, "{}", self.token.op),
                PathToken {
                    tag: Tag::F,
                    token: _PathToken { v: _ },
                } => write!(f, "{}", self.token.v),
            }
        }
    }
}

fn draw_abbreviate_path(
    dt: &mut DrawTarget,
    start_p: &Point,
    path: Vec<PathToken>,
    line_width: &f32,
    #[allow(unused_variables)] fill_color: &OfdColor,
    stroke_color: &OfdColor,
) {
    println!("draw_abbreviate_path:, {:?}", stroke_color);
    let mut idx = 0;
    let mut pb = PathBuilder::new();
    while idx < path.len() {
        unsafe {
            match path.get(idx) {
                Some(PathToken {
                    tag: Tag::C,
                    token: _PathToken { op: 'M' },
                }) => {
                    idx += 1;
                    let x = path.get(idx).unwrap().token.v;
                    idx += 1;
                    let y = path.get(idx).unwrap().token.v;
                    pb.move_to(x, y);
                }
                Some(PathToken {
                    tag: Tag::C,
                    token: _PathToken { op: 'L' },
                }) => {
                    let mut iter = path[idx + 1..idx + 3].iter().map(|pt| pt.token.v);
                    idx += 2;
                    let x = iter.next().unwrap();
                    let y = iter.next().unwrap();
                    pb.line_to(x, y);
                }
                Some(PathToken {
                    tag: Tag::C,
                    token: _PathToken { op: 'B' },
                }) => {
                    let mut iter = path[idx + 1..idx + 7].iter().map(|pt| pt.token.v);
                    idx += 6;
                    pb.cubic_to(
                        iter.next().unwrap(),
                        iter.next().unwrap(),
                        iter.next().unwrap(),
                        iter.next().unwrap(),
                        iter.next().unwrap(),
                        iter.next().unwrap(),
                    );
                }
                Some(PathToken {
                    tag: Tag::C,
                    token: _PathToken { op: 'A' },
                }) => {
                    // pb.arc()
                }
                Some(PathToken {
                    tag: Tag::C,
                    token: _PathToken { op: 'Q' },
                }) => {
                    let mut iter = path[idx + 1..idx + 5].iter().map(|pt| pt.token.v);
                    idx += 4;

                    pb.quad_to(
                        iter.next().unwrap(),
                        iter.next().unwrap(),
                        iter.next().unwrap(),
                        iter.next().unwrap(),
                    );
                }
                Some(PathToken {
                    tag: Tag::C,
                    token: _PathToken { op: 'C' },
                }) => {}
                Some(
                    pt @ PathToken {
                        tag: Tag::C,
                        token: _,
                    },
                ) => {
                    panic!("OFD path_token [{}] invalid!", pt);
                }
                Some(_) => {}
                None => {}
            }
        }
        idx += 1;
    }
    pb.close();
    let path = pb.finish();
    let new_path = path.transform(&Transform::translation(start_p.x, start_p.y));
    dt.stroke(
        &new_path,
        &Source::Solid(SolidSource {
            r: stroke_color.r,
            g: stroke_color.g,
            b: stroke_color.b,
            a: stroke_color.a,
        }),
        &StrokeStyle {
            width: line_width.clone(),
            ..Default::default()
        },
        &DrawOptions::new(),
    );
}

fn abbreviate_data(data: &String) -> Vec<PathToken> {
    data.split_whitespace()
        .map(|s| match s {
            "M" | "L" | "B" | "A" | "Q" | "C" => PathToken {
                tag: Tag::C,
                token: _PathToken {
                    op: s.chars().next().unwrap(),
                },
            },
            v => PathToken {
                tag: Tag::F,
                token: _PathToken {
                    v: v.parse().unwrap(),
                },
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::cmp::min;
    use super::{abbreviate_data, draw_abbreviate_path};
    use super::{delta_to_vec, OfdColor};
    use crate::ofd::PhysicalBox;
    use crate::node_draw::{get_font_from_family_name, PPMM};
    use euclid::Angle;
    use font_kit::family_name::FamilyName;
    use font_kit::properties::Properties;
    use font_kit::source::SystemSource;
    use image::Pixel;
    use raqote::*;

    #[test]
    fn test_dt_matrix() {
        let mut dt = DrawTarget::new(400, 400);
        dt.fill_rect(
            0.,
            0.,
            400.,
            400.,
            &Source::Solid(SolidSource {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            }),
            &DrawOptions::new(),
        );
        let p_size = 1.0;
        dt.fill_rect(
            200. - p_size,
            200. - p_size,
            p_size * 2.,
            p_size * 2.,
            &Source::Solid(SolidSource {
                r: 0x00,
                g: 0x00,
                b: 0x00,
                a: 0xff,
            }),
            &DrawOptions::new(),
        );

        let font = get_font_from_family_name("Kaiti");
        let boundary = PhysicalBox {
            x: 150.0,
            y: 150.0,
            width: 70.0,
            height: 40.0,
        };
        //  [0.323729, -0.944243, 0.9342, 0.327214, 0.0, 0.0], text: 全
        let v = vec![
            // 0.323729, -0.944243, 0.9342, 0.327214, 0.0, 0.0
            0.323729, 0.944243, -0.9342, 0.327214, 0.0, 0.0,
        ];
        let ctm = Transform::new(v[0], v[1], v[2], v[3], v[4], v[5]);
        let mm = Transform::identity()
            .pre_translate(Vector::new(-boundary.x, -boundary.y))
            .then(&ctm)
            .then_translate(Vector::new(boundary.x, boundary.y));
        dt.fill_rect(
            boundary.x,
            boundary.y,
            boundary.width,
            boundary.height,
            &Source::Solid(SolidSource {
                r: 0x34,
                g: 0x98,
                b: 0xb2,
                a: 0xff,
            }),
            &DrawOptions::new(),
        );
        dt.set_transform(&mm);
        dt.fill_rect(
            boundary.x,
            boundary.y,
            boundary.width,
            boundary.height,
            &Source::Solid(SolidSource {
                r: 0xff,
                g: 0x0,
                b: 0xff,
                a: 0xff,
            }),
            &DrawOptions::new(),
        );
        let point_size = 3.;
        let ids = vec![font.glyph_for_char('全').unwrap()];
        let positions = vec![Point::new(150., 150.)];
        let options = DrawOptions::new();
        // dt.set_transform(&Transform::identity());

        dt.set_transform(&Transform::identity());
        dt.draw_glyphs(
            &font,
            24.,
            &ids,
            &positions,
            &Source::Solid(SolidSource {
                r: 0x34,
                g: 0x98,
                b: 0xb2,
                a: 0xff,
            }),
            &DrawOptions::new(),
        );

        let t = Transform::identity()
            .then_translate(Vector::new(-200., -200.))
            .then_rotate(Angle::degrees(45.))
            .then_translate(Vector::new(200., 200.));
        dt.set_transform(&t);
        dt.draw_glyphs(
            &font,
            24.,
            &ids,
            &vec![Point::new(200., 200.)],
            &Source::Solid(SolidSource {
                r: 0x34,
                g: 0x98,
                b: 0xb2,
                a: 0xff,
            }),
            &DrawOptions::new(),
        );

        dt.set_transform(&Transform::identity());
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

        dt.fill_rect(
            0.,
            0.,
            400.,
            400.,
            &Source::Solid(SolidSource {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            }),
            &DrawOptions::new(),
        );

        let line_width: f32 = 0.5;
        let fill_color = OfdColor {
            r: 156,
            g: 82,
            b: 32,
            a: 255,
        };
        let stroke_color = OfdColor {
            r: 156,
            g: 82,
            b: 32,
            a: 255,
        };
        draw_abbreviate_path(
            &mut dt,
            &Point::new(0., 0.),
            tokens,
            &line_width,
            &fill_color,
            &stroke_color,
        );
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
        dt.fill_rect(
            0.,
            0.,
            400.,
            400.,
            &Source::Solid(SolidSource {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            }),
            &DrawOptions::new(),
        );

        let mut pb = PathBuilder::new();

        pb.move_to(0., 0.);
        pb.line_to(100., 0.);
        pb.line_to(100., 100.);
        pb.line_to(0., 100.);
        pb.close();
        dt.set_transform(&Transform::translation(60., 60.));
        dt.stroke(
            &pb.finish(),
            &OfdColor::default().solid_source(),
            &StrokeStyle {
                width: 1.,
                ..Default::default()
            },
            &DrawOptions::new(),
        );
        let t = dt.get_transform().pre_scale(5., 5.);
        dt.set_transform(&t);
        let mut pb = PathBuilder::new();

        pb.move_to(0., 0.);
        pb.line_to(100., 0.);
        pb.line_to(100., 100.);
        pb.line_to(0., 100.);
        pb.close();
        dt.stroke(
            &pb.finish(),
            &OfdColor::default().solid_source(),
            &StrokeStyle {
                width: 1.,
                ..Default::default()
            },
            &DrawOptions::new(),
        );

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
        let font = get_font_from_family_name("Kaiti");
        println!("font: {:?}", font.family_name());

        let mut dt = DrawTarget::new(400, 400);
        dt.fill_rect(
            0.,
            0.,
            400.,
            400.,
            &Source::Solid(SolidSource {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            }),
            &DrawOptions::new(),
        );
        dt.draw_text(
            &font,
            24.,
            "我",
            Point::new(0., 100.),
            &Source::Solid(SolidSource {
                r: 0,
                g: 0,
                b: 0xff,
                a: 0xff,
            }),
            &DrawOptions::new(),
        );
        dt.write_png("test_font.png").expect("");
    }

    #[test]
    fn test_select_by_postscript() {
        let font = SystemSource::new()
            .select_by_postscript_name("kai")
            .unwrap()
            .load()
            .unwrap();
        println!("font: {:?}", font);
    }

    #[test]
    fn test_map_vec() {
        let v1: Vec<u8> = vec![1, 2, 3];
        let v2: Vec<u32> = v1.into_iter().map(Into::<u32>::into).collect();

        // for i in 0..3 {
        //     v2[i] = v1[i] as u32;
        // }
        println!("{:?}", v2.len());
    }

    #[test]
    fn test_draw_image() {
        let scale: f32 = 2.0;
        let p_box = PhysicalBox {
            x: 0.0,
            y: 0.0,
            width: 210. * scale,
            height: 297. * scale,
        };
        let mut dt = DrawTarget::new(p_box.width as i32, p_box.height as i32);
        dt.fill_rect(
            0.,
            0.,
            p_box.width,
            p_box.height,
            &Source::Solid(SolidSource {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            }),
            &DrawOptions::new(),
        );
        // dt.set_transform(&Transform::scale(scale, scale));
        // let png_path = "pngs/smls_icon_355x237.png";
        let png_path = "pngs/image_7049.png";
        let img = image::open(png_path)
            .expect("convert to DynamicImage failed")
            .into_rgba8();

        println!("img: {:?}, {:x}", img[(0, 0)], u32::MAX);
        let d1: Vec<u32> = img
            .pixels()
            .into_iter()
            .map(|p| {
                // if p == &image::Rgba([255, 255, 255, 0]) {
                //     return 0;
                // }
                // Color::new(p[3], p[0], p[1], p[2])
                // ((p[0] as u32) << 24) | ((p[1] as u32) << 16) | ((p[2] as u32) << 8) | (p[3] as u32)
                // ((p[3] as u32) << 24) | ((p[2] as u32) << 16) | ((p[1] as u32) << 8) | (p[0] as u32)
                // if p[3] == 0 {
                //     return 0;
                // }
                ((p[3] as u32) << 24) | ((p[0] as u32) << 16) | ((p[1] as u32) << 8) | (p[2] as u32)
            })
            .collect();
        println!("d1: 0x{:08x}, 0x{:08x}", d1[0], u32::MAX);
        let image = Image {
            width: img.width() as i32,
            height: img.height() as i32,
            data: &d1,
        };
        // assert_eq!(d1[0], 4294967040);
        // dt.draw_image_with_size_at(360.0, 240.0, 0.0, 0.0, &image, &DrawOptions::new());
        dt.draw_image_at(0.0, 0.0, &image, &DrawOptions::new());
        dt.write_png("test_draw_image.png").expect("save file failed");
    }

    use sw_composite::{over, over_exact};

    fn over_(src: u32, dst: u32) -> u32 {
        let a = src>>24;
        let a = 256 - a;
        let mask = 0xff00ff;

        let rb = ((dst & 0xff00ff) * a) >> 8;
        let ag = ((dst >> 8) & 0xff00ff) * a;
        src + ((rb & mask) | (ag & !mask))
    }

    #[test]
    fn test_blend_over() {
        let src: u32 = 0x00f20a01;
        // let src: u32 = 0x81004000;
        let dst: u32 = 0xffffffff;

        println!("src: 0x{:x}, dst: 0x{:x}, over: 0x{:x}", src, dst, over_(src, dst));
        println!("src: 0x{:x}, dst: 0x{:x}, over: 0x{:x}", src, dst, over_exact(src, dst));
    }
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
            }
            v => vec.push(v.parse().unwrap()),
        }
    }
    vec
}
