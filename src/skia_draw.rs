use skia_safe::Canvas;
use crate::backends::{DrawBackend, Transform};
use crate::ofd::{Appearance, ImageObject, PathObject, PhysicalBox, TextObject};

pub struct SkiaBackend {
    canvas: Canvas,
}

impl SkiaBackend {
    pub fn new() -> Self {
        todo!()
    }
}

impl DrawBackend for SkiaBackend {
    fn output_page(&mut self, out_f_name: &String) -> crate::backends::Result<()> {
        todo!()
    }

    fn draw_boundary(&mut self, boundary: &PhysicalBox) {
        todo!()
    }

    fn save(&mut self) -> Transform {
        todo!()
    }

    fn restore(&mut self, transform: &Transform) {
        todo!()
    }

    fn draw_path_object(&mut self, draw_param_id: Option<&String>, path_object: &PathObject) {
        todo!()
    }

    fn draw_text_object(&mut self, draw_param_id: Option<&String>, text_object: &TextObject) {
        todo!()
    }

    fn draw_image_object(&mut self, image_object: &ImageObject) {
        todo!()
    }
}