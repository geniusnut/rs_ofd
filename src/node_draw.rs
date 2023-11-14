#![allow(dead_code)]

use crate::ofd::{PhysicalBox, DrawParam};
use font_kit::family_name::FamilyName;
use font_kit::font::Font;
use font_kit::properties::{Properties, Weight};
use font_kit::source::SystemSource;
use image::RgbaImage;
use lazy_static::lazy_static;
use send_wrapper::SendWrapper;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Mutex;

const SONGTI_LIST: &[&str] = &["SimSun", "NSimSun", "Songti", "STSong", "SimSong", "FangSong", "Songti SC"];
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
    pub static ref MUTEX_IMAGE_PNG_RES: Mutex<HashMap<String, Vec<u8>>> = {
        let m = HashMap::new();
        Mutex::new(m)
    };

    pub static ref MUTEX_RES_DRAW_PARAMS: Mutex<HashMap<String, DrawParam>> = {
        let m = HashMap::new();
        Mutex::new(m)
    };

    pub static ref RES_FONT_FAMILY_NAME_MAP: Mutex<HashMap<String, String>> = {
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
    // println!("family_name: {}, candidates: {:?}", family_name, k);
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

pub fn boundary_to_rect(boundary: &String) -> PhysicalBox {
    let mut iter = boundary.split_whitespace().into_iter();
    PhysicalBox {
        x: iter.next().unwrap().parse().unwrap(),
        y: iter.next().unwrap().parse().unwrap(),
        width: iter.next().unwrap().parse().unwrap(),
        height: iter.next().unwrap().parse().unwrap(),
    }
}

#[derive(Debug, Clone)]
pub struct OfdColor {
    pub(crate) r: u8,
    pub(crate) g: u8,
    pub(crate) b: u8,
    pub(crate) a: u8,
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


pub fn ofd_color_from_v(s: &String) -> OfdColor {
    let mut iter = s.split_whitespace().into_iter();
    OfdColor {
        r: iter.next().unwrap().parse().unwrap(),
        g: iter.next().unwrap().parse().unwrap(),
        b: iter.next().unwrap().parse().unwrap(),
        a: iter.next().unwrap_or("255").parse().unwrap(),
    }
}


pub union _PathToken {
    pub op: char,
    pub v: f32,
}

pub enum Tag {
    C,
    F,
}

pub struct PathToken {
    pub tag: Tag,
    pub token: _PathToken,
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

pub fn abbreviate_data(data: &String) -> Vec<PathToken> {
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


pub fn delta_to_vec(data: &String) -> Vec<f32> {
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

pub fn get_color_from_draw_param(draw_param_id: Option<&String>) -> (Option<OfdColor>, Option<OfdColor>) {
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
    (dp_fill_color, dp_stroke_color)
}