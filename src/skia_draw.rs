use std::fs::File;
use std::io::{Write};

use skia_safe::path::ArcSize;
use skia_safe::{Color, Data, EncodedImageFormat, Font, FontStyle, Image, Matrix, Paint, paint, PaintStyle, Path, PathDirection, Point, Rect, Surface, surfaces, TextBlob, Typeface};

use crate::backends::{DrawBackend, Transform};
use crate::backends::DrawError::OutputError;
use crate::node_draw::{_PathToken, abbreviate_data, delta_to_vec, get_color_from_draw_param, MUTEX_IMAGE_PNG_RES, MUTEX_IMAGE_RES, MUTEX_RES_DRAW_PARAMS, ofd_color_from_v, OfdColor, PathToken, PPMM, RES_FONT_ID_MAP, Tag};
use crate::ofd::{ImageObject, PathObject, PhysicalBox, TextObject};

pub struct SkiaBackend {
    pub surface: Surface,
    path: Path,
    paint: Paint,
}

impl From<Matrix> for Transform {
    fn from(matrix: Matrix) -> Self {
        Transform {
            m11: matrix.scale_x(),
            m12: matrix.skew_x(),
            m21: matrix.translate_x(),
            m22: matrix.scale_y(),
            m31: matrix.skew_y(),
            m32: matrix.translate_y(),
        }
    }
}

impl From<OfdColor> for Color {
    fn from(ofd_color: OfdColor) -> Self {
        Color::from_argb(ofd_color.a, ofd_color.r, ofd_color.g, ofd_color.b)
    }
}

struct AdapterCtm(Option<String>);

impl AdapterCtm {
    fn to_matrix(&self) -> Matrix {
        self.0.clone().map_or(
            *Matrix::i(),
            |s| {
                let vec: Vec<f32> = s.split_whitespace().map(|s| s.parse().unwrap()).collect();
                Matrix::new_all(
                    vec[0], vec[2], vec[4],
                    vec[1], vec[3], vec[5],
                    0.0, 0.0, 1.0
                )
            })
    }
}

impl SkiaBackend {
    pub fn new(width: i32, height: i32) -> Self {
        let mut surface = surfaces::raster_n32_premul((width, height)).expect("surface");
        let path = Path::new();
        let mut paint = Paint::default();
        paint.set_color(Color::BLACK);
        paint.set_anti_alias(true);
        paint.set_stroke_width(1.0);
        surface.canvas().clear(Color::WHITE);
        surface.canvas().scale((PPMM, PPMM));
        SkiaBackend {
            surface,
            path,
            paint,
        }
    }
}

impl DrawBackend for SkiaBackend {
    fn output_page(&mut self, out_f_name: &String) -> crate::backends::Result<()> {
        println!("output page: {}", out_f_name);
        let image = self.surface.image_snapshot();
        let mut context = self.surface.direct_context();
        let d = image.encode(context.as_mut(), EncodedImageFormat::PNG, None)
            .unwrap();
        let mut file = File::create(out_f_name).unwrap();
        let bytes = d.as_bytes();
        file.write_all(bytes).map_err(|e|
            OutputError(format!("write png file {} failed: {}", out_f_name, e))
        )
    }

    fn draw_boundary(&mut self, boundary: &PhysicalBox) {
        // println!("draw_boundary: {:?}", boundary);
        self.surface.canvas().translate((boundary.x, boundary.y));
    }

    fn save(&mut self) -> Transform {
        self.surface.canvas().save();
        self.surface.canvas().local_to_device_as_3x3().into()
    }

    fn scale(&mut self) {
        self.surface.canvas().scale((PPMM, PPMM));
    }

    fn restore(&mut self, _transform: &Transform) {
        self.surface.canvas().restore();
    }

    fn draw_path_object(&mut self, draw_param_id: Option<&String>, path_object: &PathObject) {
        self.surface.canvas().save();
        draw_path_object(&mut self.surface, draw_param_id, path_object);
        self.surface.canvas().restore();
    }

    fn draw_text_object(&mut self, draw_param_id: Option<&String>, text_object: &TextObject) {
        self.surface.canvas().save();
        draw_text_object(&mut self.surface, draw_param_id, text_object);
        self.surface.canvas().restore();
    }

    fn draw_image_object(&mut self, image_object: &ImageObject) {
        // println!("draw_image_object: {:#?}", image_object);
        let _id  = image_object.id.clone();
        let img_file = MUTEX_IMAGE_RES.lock().unwrap().get(image_object.resource_id.as_str()).unwrap().clone();
        let png_data = MUTEX_IMAGE_PNG_RES.lock().unwrap().get(&img_file).unwrap().clone();

        let image = Image::from_encoded(Data::new_copy(png_data.as_slice())).unwrap();
        // println!("image: {:?}", image);

        let boundary = image_object.boundary.clone();
        self.surface.canvas().draw_image_rect(
            image,
            None,
            Rect::from_point_and_size((boundary.x, boundary.y), (boundary.width, boundary.height)),
            &Paint::default());
    }
}

fn draw_text_object(surface: &mut Surface, draw_param_id: Option<&String>, text_object: &TextObject) {
    let (dp_fill_color, _dp_stroke_color) = get_color_from_draw_param(draw_param_id);

    let boundary = text_object.boundary;
    let font_id = text_object.font.clone();
    let size: f32 = text_object.size;
    let fill_color = text_object.fill_color.clone().map_or(
        dp_fill_color.clone().unwrap_or(OfdColor::default()),
        |s| match s.value {
            None => dp_fill_color.clone().unwrap_or(OfdColor::default()),
            Some(c) => ofd_color_from_v(&c)
        }
    );
    let font = RES_FONT_ID_MAP.lock().unwrap().get(font_id.as_str()).unwrap().clone().take();

    // println!("draw_text_object {:?}, {:?}: {:?}", &dp_fill_color, &fill_color, text_object);

    let ctm: Matrix = AdapterCtm(text_object.ctm.clone()).to_matrix();
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
    let point_size = size;
    let font = Font::from_typeface_with_params(
        Typeface::new(font.family_name(), FontStyle::default())
            .unwrap_or(Typeface::default()),
        point_size, 1.0, 0.0,
    );

    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(fill_color);
    paint.set_style(PaintStyle::Fill);

    let mut pos = start_p.clone();

    surface.canvas().translate((boundary.x, boundary.y));
    surface.canvas().concat(&ctm);
    surface.canvas().translate((-boundary.x, -boundary.y));

    for c in text_code.text.chars() {
        let mut text = String::new();
        text.push(c);
        positions.push(pos);
        // println!("draw text: {} with {:?}", text, pos);
        let blob = TextBlob::new(text, &font).unwrap();
        surface.canvas().draw_text_blob(
            blob,
            (pos.x, pos.y),
            &paint,
        );

        pos.x += iter_delta_x.next().unwrap_or(0.);
        pos.y += iter_delta_y.next().unwrap_or(0.);
    }
}

fn draw_path_object(surface: &mut Surface, draw_param_id: Option<&String>, path_object: &PathObject) {
    let draw_param = draw_param_id.map_or(
        None,
        |it| {
            Some(MUTEX_RES_DRAW_PARAMS.lock().unwrap().get(it).unwrap().clone())
        }
    );
    let line_width: f32 = path_object.line_width.unwrap_or(0.5);
    let boundary = &path_object.boundary;

    // println!("draw_path_object: {:?}", path_object);
    let path = abbreviate_data(&path_object.abbreviated_data);
    // vec[0], -vec[1], -vec[2], vec[3], vec[4], vec[5],
    let ctm: Matrix = AdapterCtm(path_object.ctm.clone()).to_matrix();

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

    surface.canvas().translate((boundary.x, boundary.y));
    surface.canvas().concat(&ctm);

    let mut new_path = Path::new();
    let mut idx = 0;
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
                    new_path.move_to((x, y));
                }
                Some(PathToken {
                         tag: Tag::C,
                         token: _PathToken { op: 'L' },
                     }) => {
                    let mut iter = path[idx + 1..idx + 3].iter().map(|pt| pt.token.v);
                    idx += 2;
                    let x = iter.next().unwrap();
                    let y = iter.next().unwrap();
                    new_path.line_to((x, y));
                }
                Some(PathToken {
                         tag: Tag::C,
                         token: _PathToken { op: 'B' },
                     }) => {
                    let mut iter = path[idx + 1..idx + 7].iter().map(|pt| pt.token.v);
                    idx += 6;
                    new_path.cubic_to(
                        (iter.next().unwrap(), iter.next().unwrap()),
                        (iter.next().unwrap(), iter.next().unwrap()),
                        (iter.next().unwrap(), iter.next().unwrap()),
                    );
                }
                Some(PathToken {
                         tag: Tag::C,
                         token: _PathToken { op: 'A' },
                             }) => {
                            let mut iter = path[idx + 1..idx + 8].iter().map(|pt| pt.token.v);
                            idx += 7;
                            let (rx, ry) = (iter.next().unwrap(), iter.next().unwrap());
                            let x_axis_rotate = iter.next().unwrap();
                            let large_arc = if (iter.next().unwrap() as u32) > 0 { ArcSize::Large } else { ArcSize::Small };
                            let sweep = if (iter.next().unwrap() as i32) > 0 {PathDirection::CW} else {PathDirection::CCW};
                            let (end_x, end_y) = (iter.next().unwrap(), iter.next().unwrap());

                            new_path.arc_to_rotated((rx, ry), x_axis_rotate, large_arc, sweep, (end_x, end_y));
                        }
                Some(PathToken {
                         tag: Tag::C,
                         token: _PathToken { op: 'Q' },
                     }) => {
                    let mut iter = path[idx + 1..idx + 5].iter().map(|pt| pt.token.v);
                    idx += 4;

                    new_path.quad_to(
                        (iter.next().unwrap(), iter.next().unwrap()),
                        (iter.next().unwrap(), iter.next().unwrap()),
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
    new_path.close();
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_stroke_width(line_width);
    paint.set_color(stroke_color);
    paint.set_style(paint::Style::Stroke);
    surface.canvas().draw_path(&new_path, &paint);
}


#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;

    use skia_safe::{Color, Paint, Rect, surfaces};

    #[test]
    fn test_ofd_color() {
        use crate::skia_draw::OfdColor;
        let mut c = OfdColor::default();
        c.r = 127;
        let mut paint = Paint::default();
        paint.set_color(c);
        println!("paint: {:?}", paint);

        let (width, height) = (400, 400);
        let mut surface = surfaces::raster_n32_premul((width, height)).expect("surface");
        surface.canvas().clear(Color::WHITE);

        let mut stack = surface.canvas().save();
        assert_eq!(stack, 1);
        surface.canvas().scale((2.0, 2.0));
        surface.canvas().translate((50, 50));

        stack = surface.canvas().save();
        println!("save 2 stack: {stack}");
        surface.canvas().translate((10, 10));
        surface.canvas().draw_rect(Rect::from_point_and_size((0, 0), (100, 100)), &paint);
        surface.canvas().restore();

        stack = surface.canvas().save();
        println!("save 2 stack: {stack}");
        surface.canvas().translate((100, 100));
        paint.set_color(Color::RED);
        surface.canvas().draw_rect(Rect::from_point_and_size((0, 0), (100, 100)), &paint);
        surface.canvas().restore();

        surface.canvas().restore();

        stack = surface.canvas().save();
        println!("save 1 stack: {stack}");
        paint.set_color(Color::BLACK);
        surface.canvas().translate((10, 10));
        surface.canvas().draw_rect(Rect::from_point_and_size((0, 0), (100, 100)), &paint);
        surface.canvas().restore();

        println!("stack: {stack}");
        paint.set_color(Color::GREEN);
        surface.canvas().draw_circle((120, 120), 5.0, &paint);
        surface.canvas().draw_circle((300, 300), 5.0, &paint);
        surface.canvas().draw_circle((10, 10), 5.0, &paint);

        let image = surface.image_snapshot();
        let d = image.encode(surface.direct_context().as_mut(),
                             skia_safe::EncodedImageFormat::PNG, None).unwrap();

        let mut file = File::create("test.png").unwrap();
        let bytes = d.as_bytes();
        file.write_all(bytes).unwrap();
    }
}