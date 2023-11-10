use std::result;
use crate::raqote_draw::RaqoteDrawBackend;
use crate::skia_draw::SkiaBackend;
use crate::ofd::{Appearance, ImageObject, PathObject, PhysicalBox, TextObject};

pub type Result<T> = result::Result<T, DrawError>;

#[derive(Debug)]
pub enum DrawError {
    DrawingError(String),
    OutputError(String),
}

pub struct Transform {
    pub(crate) m11: f32, pub(crate) m12: f32,
    pub(crate) m21: f32, pub(crate) m22: f32,
    pub(crate) m31: f32, pub(crate) m32: f32,
}

impl Transform {
    pub(crate) fn identity() -> Self {
        Transform {
            m11: 1.0, m12: 0.0,
            m21: 0.0, m22: 1.0,
            m31: 0.0, m32: 0.0,
        }
    }
}

impl From<raqote::Transform> for Transform {
    fn from(value: raqote::Transform) -> Self {
        Transform {
            m11: value.m11, m12: value.m12,
            m21: value.m21, m22: value.m22,
            m31: value.m31, m32: value.m32,
        }
    }
}

// impl Into<raqote::Transform> for Transform {
//     fn into(self) -> raqote::Transform {
//         raqote::Transform::new(self.m11, self.m12, self.m21, self.m22, self.m31, self.m32)
//     }
// }

pub trait DrawBackend {
    fn output_page(&mut self, out_f_name: &String) -> Result<()>;

    fn draw_boundary(&mut self, boundary: &PhysicalBox);
    fn save(&mut self) -> Transform;

    fn scale(&mut self);

    fn restore(&mut self, transform: &Transform);

    fn draw_path_object(&mut self, draw_param_id: Option<&String>, path_object: &PathObject);
    fn draw_text_object(&mut self, draw_param_id: Option<&String>, text_object: &TextObject);
    fn draw_image_object(&mut self, image_object: &ImageObject);
}

pub fn new_draw_backend(width: i32, height: i32) -> Box<dyn DrawBackend> {
    if cfg!(feature = "raqote") {
        Box::new(RaqoteDrawBackend::new(width, height))
    } else {
        Box::new(SkiaBackend::new(width, height))
    }
}