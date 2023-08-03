#![allow(dead_code)]
use zip::read::ZipFile;
use zip::result::ZipError;
use xmltree::Element;

use font_kit::family_name::FamilyName;
use font_kit::font::Font;
use font_kit::properties::{Properties, Weight};
use font_kit::source::SystemSource;


pub const OFD_XML: &'static str = "OFD.xml";
const OFD_NAMESPACE_URL: &'static str = "http://www.ofdspec.org/2016";
// const DOC_0: &'static str = "Doc_0";


#[derive(Debug)]
pub struct OFDFile {
    pub doc_body: Option<Element>,
    pub doc_root: String,
    pub ofd_doc: Option<OFDDoc>,
}

impl Default for OFDFile {
    fn default() -> Self {
        OFDFile {
            doc_body: None,
            doc_root: String::new(),
            ofd_doc: None,
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct PhysicalBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}


#[derive(Debug, Clone)]
pub struct OFDDoc {
    pub template_page: Option<String>,
    pub physical_box: Option<PhysicalBox>,
    pub document_res: Option<String>,
    pub public_res: Option<String>,
    pub pages: Vec<OFDPage>,
    pub annotations: Option<String>,
    pub attachment: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct OFDPage {}

#[derive(Debug)]
pub enum OFDError {
    ZipError(ZipError),
    CustomError(String),
}

#[derive(Debug, Clone)]
pub struct OFDFont {
    pub id: String,
    pub font_name: String,
    pub family_name: String,
    pub sys_font: Font,
}

impl OFDFont {
    fn new(id: String, font_name: String, family_name: String) -> Self {
        let  sys_font = SystemSource::new().select_best_match(
            &[FamilyName::Title(family_name.clone())],
            &Properties::new().weight(Weight::NORMAL),
        ).unwrap()
            .load()
            .unwrap();
        println!("OFDFont[{id}, {font_name}, {family_name}], {:?}", sys_font);
        OFDFont {
            id,
            font_name,
            family_name,
            sys_font,
        }
    }
}

enum OFDResource {
    DrawParam(),
    MultiMedia(),
    Font(),
}

enum OFDMMType {
    Image(),
}

pub enum OFDImageFormat {
    GBIG2(),
    PNG(),
    JPG(),
}

pub struct OFDMMImage {
    pub mm_format: OFDImageFormat,
    pub file_name: String,
    pub file_path: String,
}

impl OFDMMImage {
    fn new(mm_format: OFDImageFormat, file_name: String) -> OFDMMImage {
        OFDMMImage {
            mm_format,
            file_name,
            file_path: "".to_string(),
        }
    }
}

type OFDResult<T> = Result<T, ()>;

impl OFDFile {
    pub fn read_xml_tree(&mut self, zf: &mut ZipFile) {
        let ofd_element = Element::parse(zf).unwrap();
        // println!("ofd_element: {}", ofd_element.name);
        // for child in &ofd_element.children[0].as_element().unwrap().children {
        //     println!("element: {:?}", child.as_element().unwrap().children);
        // }
        self.doc_body = Some(ofd_element.children[0].as_element().unwrap().clone());
        println!("DocBody: {:?}", self.doc_body);
    }
}
