#![allow(dead_code)]

use std::collections::HashMap;
use crate::node_draw::{boundary_to_rect, draw_ele_recursive, draw_image_object, draw_path_object, draw_text_object, get_font_from_family_name, MUTEX_IMAGE_RES, MUTEX_RES_DRAW_PARAMS, MUTEX_RGB_IMAGE_RES, PPMM, RES_FONT_ID_MAP};
use font_kit::family_name::FamilyName;
use font_kit::font::Font;
use font_kit::properties::{Properties, Weight};
use font_kit::source::SystemSource;
use jbig2dec::Document;
use raqote::{DrawOptions, DrawTarget, SolidSource, Source, Transform, Vector};
use send_wrapper::SendWrapper;
use serde::{Deserialize, Deserializer, Serialize};
use std::fs::File;
use std::io;
use std::io::{Cursor, Read, Seek};
use std::path::Path;
use std::ptr::hash;
use xmltree::Element;
use zip::read::ZipFile;
use zip::result::ZipError;
use zip::{read, ZipArchive};
use crate::backends;
use crate::backends::{DrawBackend};

pub const OFD_XML: &'static str = "OFD.xml";
pub const OFD_NAMESPACE_URL: &'static str = "http://www.ofdspec.org/2016";
// const DOC_0: &'static str = "Doc_0";

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PhysicalBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug)]
pub struct OFDFile {
    archive: ZipArchive<File>,
    file_name: String,
    pub doc_body: Option<Element>,
    pub doc_root: String,
    pub ofd_doc: Option<OFDDoc>,
}

impl OFDFile {
    #[allow(unused_mut)]
    pub fn new(file_name: &str) -> Self {
        let file_path = Path::new(file_name);
        let out_f_name = file_path.with_extension("");

        let file = File::open(&file_path).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        let mut ofd_file = OFDFile {
            archive,
            file_name: out_f_name.to_str().unwrap().to_string(),
            doc_body: None,
            doc_root: String::new(),
            ofd_doc: None,
        };
        ofd_file.inflate();
        ofd_file.parse_resources();
        ofd_file
    }

    fn inflate(&mut self) {
        let mut doc_body: Option<Element> = None;
        let file = self.archive.by_name(OFD_XML).unwrap();
        let ofd_element = Element::parse(file).unwrap();

        doc_body = Some(ofd_element.children[0].as_element().unwrap().clone());

        let doc_body_v = doc_body.unwrap();
        let info = doc_body_v.get_child("DocInfo").expect("DocInfo not found");
        // println!("DocInfo: {:?}", info);

        let doc_root = String::from(
            doc_body_v
                .get_child("DocRoot")
                .expect("DocRoot not found")
                .get_text()
                .unwrap(),
        );
        self.doc_root = Path::new(&doc_root).parent().unwrap().to_str().unwrap().to_string();
        let doc_root_path = Path::new(&doc_root).parent().unwrap();
        // println!("doc_root_path: {:?}", doc_root_path);

        let mut file = self.archive.by_name(&doc_root).unwrap();
        let mut s = String::new();
        let _ = file.read_to_string(&mut s);
        drop(file);

        let ofd_document: OFDDocument = quick_xml::de::from_str(s.as_str()).unwrap();
        // println!("ofd_document: {:?}", ofd_document);

        self.ofd_doc = Some(OFDDoc {
            doc_root_path: doc_root_path.to_str().unwrap().to_string(),
            template_pages: ofd_document.common_data.template_page.into_iter().map(|s|
                TemplatePage {
                    id: s.id,
                    base_loc: format!("{}/{}", doc_root_path.to_str().unwrap(), s.base_loc),
                }
            ).collect(),
            physical_box: ofd_document.common_data.page_area.map_or(None, |s| {
                Some(s.physical_box)
            }),
            document_res: OFDRes::new(
                &mut self.archive,
                format!("{}/{}", doc_root_path.to_str().unwrap(), ofd_document.common_data.document_res)
            ),
            public_res: OFDRes::new(
                &mut self.archive,
                format!("{}/{}", doc_root_path.to_str().unwrap(), ofd_document.common_data.public_res)
            ),
            pages: ofd_document.pages.page.into_iter().enumerate().map(|(idx, page)|
                OFDPage::new(
                    &mut self.archive,
                    format!("{}/{}", doc_root_path.to_str().unwrap(), page.base_loc).as_str(),
                    idx,
                    page.id,
                )
            ).collect(),
            annotations: OFDAnnotations::new(
                &mut self.archive,
                format!("{}/{}", doc_root_path.to_str().unwrap(), ofd_document.annotations)
            ),
            attachment: ofd_document.attachments,
        });
        // println!("ofd_doc: {:?}", self.ofd_doc);
    }

    fn parse_resources(&mut self) {
        let ofd_doc = self.ofd_doc.as_mut().unwrap();
        for multimedia in &ofd_doc.document_res.multi_medias {
            if multimedia.type_.eq("Image") {
                MUTEX_IMAGE_RES.lock().unwrap().insert(
                    multimedia.id.clone(),
                    multimedia.media_file.text.clone()
                );
            }
        }

        for font in &ofd_doc.public_res.fonts {
            let family_name = font.family_name.clone().unwrap_or(font.font_name.clone());
            RES_FONT_ID_MAP.lock().unwrap().insert(
                font.id.clone(),
                SendWrapper::new(get_font_from_family_name(family_name.as_str())),
            );
        }
        println!("RES_FONT_ID_MAP: {:?}", RES_FONT_ID_MAP.lock().unwrap());

        let mut hashmap = HashMap::new();
        for draw_param in &ofd_doc.public_res.draw_params {
            MUTEX_RES_DRAW_PARAMS.lock().unwrap().insert(
                draw_param.id.clone(),
                draw_param.clone()
            );
            hashmap.insert(draw_param.id.clone(), draw_param);
        }
        for draw_param in &ofd_doc.document_res.draw_params {
            MUTEX_RES_DRAW_PARAMS.lock().unwrap().insert(
                draw_param.id.clone(),
                draw_param.clone()
            );
            hashmap.insert(draw_param.id.clone(), draw_param);
        }
        for draw_param in MUTEX_RES_DRAW_PARAMS.lock().unwrap().values_mut() {
            println!("expand draw_param: {:?}", draw_param.id);
            if let Some(relative) = draw_param.relative.clone() {
                let relative_draw_param = hashmap.get(&relative).unwrap().clone();
                println!("relative: {:?}", relative);
                draw_param.update(&relative_draw_param);
            }
        }
        println!("MUTEX_RES_DRAW_PARAMS: {:?}", MUTEX_RES_DRAW_PARAMS.lock().unwrap());

        for i in 0..self.archive.len() {
            let mut file = self.archive.by_index(i).unwrap();
            for v in MUTEX_IMAGE_RES.lock().unwrap().values() {
                if (*file.name()).ends_with(v) {
                    let mut buf: Vec<u8> = Vec::new();
                    let _ = &file.read_to_end(&mut buf).unwrap();
                    if (*file.name()).ends_with(".png") {
                        let dyn_image = image::load_from_memory(&buf)
                            .expect("convert to DynamicImage failed")
                            .into_rgba8();
                        MUTEX_RGB_IMAGE_RES
                            .lock()
                            .unwrap()
                            .insert(String::from(v), dyn_image);
                    } else if (*file.name()).ends_with(".jpg") || (*file.name()).ends_with(".jpeg")
                    {
                        let dyn_image = image::load_from_memory(&buf)
                            .expect("convert to DynamicImage failed")
                            .into_rgba8();
                        MUTEX_RGB_IMAGE_RES
                            .lock()
                            .unwrap()
                            .insert(String::from(v), dyn_image);
                    } else if (*file.name()).ends_with(".jb2") {
                        let mut buff = Cursor::new(buf);
                        let doc = Document::from_reader(&mut buff).expect("");
                        let image = doc.images().get(0).unwrap().clone();
                        let img = image.to_png().unwrap();
                        let dyn_image = image::load_from_memory(&img)
                            .expect("convert to DynamicImage failed")
                            .into_rgba8();
                        MUTEX_RGB_IMAGE_RES
                            .lock()
                            .unwrap()
                            .insert(String::from(v), dyn_image);
                    }
                }
            }
        }
    }

    pub fn draw(&mut self) {
        let ofd_doc = self.ofd_doc.clone().expect("ofd_doc is None");
        ofd_doc.draw_pages(&mut self.archive, self.file_name.as_str());
    }
}

#[derive(Debug, Clone)]
pub struct OFDDoc {
    pub doc_root_path: String,
    pub template_pages: Vec<TemplatePage>,
    pub physical_box: Option<PhysicalBox>,
    pub document_res: OFDRes,
    pub public_res: OFDRes,
    pub pages: Vec<OFDPage>,
    pub annotations: OFDAnnotations,
    pub attachment: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OFDAnnotations {
    #[serde(skip_deserializing)]
    dir: String,
    #[serde(rename = "Page")]
    annotations: Vec<OFDAnnotation>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct OFDAnnotation {

    #[serde(rename = "@PageID")]
    page_id: String,
    file_loc: FileLoc,
}

impl OFDAnnotations {
    fn new(archive: &mut ZipArchive<File>, path: String) -> Self {
        // get dir from path
        let dir = Path::new(path.as_str()).parent().unwrap().to_str().unwrap();
        println!("OFDAnnotation.new path: {:?}", dir);

        let mut file = archive.by_name(path.as_str()).unwrap();
        let mut buf: Vec<u8> = Vec::new();
        let _ = &file.read_to_end(&mut buf).unwrap();
        let mut buf = Cursor::new(buf);
        let mut annotations: OFDAnnotations = quick_xml::de::from_reader(&mut buf).unwrap();
        annotations.dir = dir.to_string();
        annotations
    }
}

#[derive(Debug, Clone, Deserialize)]
struct PageAnnot {
    #[serde(rename = "Annot")]
    annots: Vec<Annot>,
}

impl PageAnnot {
    fn draw(&self, backend: &mut dyn DrawBackend) {
        for annot in &self.annots {
            annot.appearance.draw(backend);
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Annot {
    #[serde(rename = "@ID")]
    id: String,
    appearance: Appearance,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Appearance {
    #[serde(rename = "@Boundary")]
    #[serde(deserialize_with = "deserialize_physical_box")]
    pub boundary: PhysicalBox,

    image_object: Option<Vec<ImageObject>>,
    text_object: Option<Vec<TextObject>>,
}

impl Appearance {
    fn draw(&self, backend: &mut dyn DrawBackend) {
        let transform = backend.save();
        backend.draw_boundary(&self.boundary);

        if let Some(image_object) = &self.image_object {
            for image_object in image_object {
                backend.draw_image_object(&image_object);
            }
        }
        if let Some(text_object) = &self.text_object {
            for text_object in text_object {
                backend.draw_text_object(None, &text_object);
            }
        }
        backend.restore(&transform);
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct _Color {
    #[serde(rename = "@Value")]
    pub value: Option<String>,
    #[serde(rename = "@ColorSpace")]
    pub color_space: Option<String>,
}


#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OFDRes {
    #[serde(rename = "@BaseLoc")]
    pub base_loc: Option<String>,

    #[serde(default, deserialize_with = "deserialize_unwrap_multi_media")]
    pub multi_medias: Vec<OFDMultiMedia>,
    #[serde(default, deserialize_with = "deserialize_unwrap_draw_params")]
    pub draw_params: Vec<DrawParam>,
    #[serde(default, deserialize_with = "deserialize_unwrap_fonts")]
    pub fonts: Vec<_Font>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct _Font {
    #[serde(rename = "@ID")]
    pub id: String,
    #[serde(rename = "@FontName")]
    pub font_name: String,
    #[serde(rename = "@FamilyName")]
    pub family_name: Option<String>,
}

impl OFDRes {
    fn new(archive: &mut ZipArchive<File>, path: String) -> OFDRes {
        let mut z_f = archive.by_name(path.as_str()).unwrap();
        let mut s = String::new();
        let _ = &z_f.read_to_string(&mut s).unwrap();
        quick_xml::de::from_str(&s).unwrap()
    }
}

fn deserialize_unwrap_multi_media<'de, D>(deserializer: D) -> Result<Vec<OFDMultiMedia>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct MultiMedias {
        #[serde(default)]
        multi_media: Vec<OFDMultiMedia>,
    }
    Ok(MultiMedias::deserialize(deserializer)?.multi_media)
}

fn deserialize_unwrap_draw_params<'de, D>(deserializer: D) -> Result<Vec<DrawParam>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct DrawParams {
        #[serde(default)]
        draw_param: Vec<DrawParam>,
    }
    Ok(DrawParams::deserialize(deserializer)?.draw_param)
}

fn deserialize_unwrap_fonts<'de, D>(deserializer: D) -> Result<Vec<_Font>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Fonts {
        #[serde(default)]
        font: Vec<_Font>,
    }
    Ok(Fonts::deserialize(deserializer)?.font)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct OFDMultiMedias {
    multi_media: Vec<OFDMultiMedia>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OFDMultiMedia {
    #[serde(rename = "@ID")]
    id: String,
    #[serde(rename = "@Type")]
    type_: String,
    #[serde(rename = "@Format")]
    format: Option<String>,

    media_file: MediaFile,
}

#[derive(Debug, Clone, Deserialize)]
struct MediaFile {
    #[serde(rename="$text")]
    text: String,
}

#[derive(Debug, Clone, Deserialize)]
struct FileLoc {
    #[serde(rename = "$text")]
    text: String,
}


#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DrawParam {
    #[serde(rename = "@ID")]
    id: String,
    #[serde(rename = "@LineWidth")]
    line_width: Option<f32>,
    #[serde(rename="@Relative")]
    pub relative: Option<String>,

    pub fill_color: Option<_Color>,
    pub stroke_color: Option<_Color>,
}

impl DrawParam {
    pub fn update(&mut self, draw_param: &DrawParam) {
        if draw_param.line_width.is_some() {
            self.line_width = draw_param.line_width;
        }
        if draw_param.fill_color.is_some() {
            self.fill_color = draw_param.fill_color.clone();
        }
        if draw_param.stroke_color.is_some() {
            self.stroke_color = draw_param.stroke_color.clone();
        }
    }
}


#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OFDPage {
    #[serde(skip_deserializing)]
    idx: usize,
    #[serde(skip_deserializing)]
    id: String,

    template: Option<_PageTemplate>,
    area: Option<Area>,
    content: OFDContent,
}


#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContentPage {
    area: Option<Area>,
    content: OFDContent,
}

#[derive(Debug, Clone, Deserialize)]
struct OFDContent {
    #[serde(rename = "$value")]
    layers: Vec<OFDLayer>,
}

impl OFDContent {
    fn draw(&self, backend: &mut dyn DrawBackend) {
        for layer in &self.layers {
            layer.draw(backend);
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OFDLayer {
    #[serde(rename = "@ID")]
    id: String,
    #[serde(rename = "@DrawParam")]
    draw_param_id: Option<String>,

    path_object: Option<Vec<PathObject>>,
    image_object: Option<Vec<ImageObject>>,
    text_object: Option<Vec<TextObject>>,
}


impl OFDLayer {
    fn draw(&self, backend: &mut dyn DrawBackend) {
        println!("draw layer with draw_param: {:?}", self.draw_param_id);
        if let Some(path_objects) = &self.path_object {
            for path_object in path_objects {
                backend.draw_path_object(self.draw_param_id.as_ref(), &path_object);
            }
        }
        if let Some(image_objects) = &self.image_object {
            for image_object in image_objects {
                backend.draw_image_object(&image_object);
            }
        }
        if let Some(text_objects) = &self.text_object {
            for text_object in text_objects {
                backend.draw_text_object(self.draw_param_id.as_ref(), &text_object);
            }
        }
    }
}


#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PathObject {
    #[serde(rename = "@ID")]
    id: String,
    #[serde(rename = "@Boundary")]
    #[serde(deserialize_with = "deserialize_physical_box")]
    pub boundary: PhysicalBox,
    #[serde(rename = "@LineWidth")]
    pub line_width: Option<f32>,
    #[serde(rename="@CTM")]
    pub ctm: Option<String>,

    pub stroke_color: Option<_Color>,
    pub fill_color: Option<_Color>,
    pub abbreviated_data: String,
}


#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TextObject {
    #[serde(rename = "@ID")]
    id: String,
    #[serde(rename = "@Boundary")]
    #[serde(deserialize_with = "deserialize_physical_box")]
    pub boundary: PhysicalBox,
    #[serde(rename = "@CTM")]
    pub ctm: Option<String>,
    #[serde(rename = "@Font")]
    pub font: String,
    #[serde(rename = "@Size")]
    pub size: f32,

    pub fill_color: Option<_Color>,
    pub stroke_color: Option<_Color>,
    pub text_code: TextCode,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TextCode {
    #[serde(rename = "@X")]
    pub x: f32,
    #[serde(rename = "@Y")]
    pub y: f32,
    #[serde(rename = "@DeltaX")]
    pub delta_x: Option<String>,
    #[serde(rename = "@DeltaY")]
    pub delta_y: Option<String>,


    #[serde(rename = "$text")]
    pub text: String,
}


#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ImageObject {
    #[serde(rename = "@ResourceID")]
    pub resource_id: String,
    #[serde(rename = "@ID")]
    pub id: String,
    #[serde(rename = "@Boundary")]
    #[serde(deserialize_with = "deserialize_physical_box")]
    pub boundary: PhysicalBox,
    #[serde(rename = "@CTM")]
    pub ctm: Option<String>,
}


#[derive(Debug, Clone, Deserialize)]
struct _PageTemplate {
    #[serde(rename = "@TemplateID")]
    id: String,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Area {
    #[serde(deserialize_with = "deserialize_physical_box")]
    physical_box: PhysicalBox,
}

impl OFDPage
{
    fn new<R: Read + Seek>(archive: &mut ZipArchive<R>, page_path: &str, idx: usize, page_id: String)
        -> Self
    {
        // println!("OFDPage new page_path: {:?}", page_path);
        let mut z_f = archive.by_name(page_path).unwrap();
        let mut s = String::new();
        let _ = &z_f.read_to_string(&mut s).unwrap();

        let mut ofd_page: OFDPage = quick_xml::de::from_str(s.as_str()).expect("Failed to parse XML");
        // println!("OFDPage new ofd_page: {:#?}", ofd_page);
        ofd_page.id = page_id;
        ofd_page.idx = idx;
        ofd_page
    }

    fn draw(&self, archive: &mut ZipArchive<File>, ofd_doc: &OFDDoc, base_name: &str) {
        let mut p_box = self.area.map_or(
            ofd_doc.physical_box.unwrap_or_default(),
            |area| area.physical_box
        );
        println!("draw page p_box: {:?}", p_box);

        p_box.width = p_box.width * PPMM;
        p_box.height = p_box.height * PPMM;

        let mut binding = backends::new_draw_backend(p_box.width as i32, p_box.height as i32);
        let mut backend = binding.as_mut();

        if self.template.is_some() {
            let template_path = ofd_doc.template_pages.clone().into_iter()
                .find(|tpl| tpl.id == self.template.as_ref().unwrap().id)
                .unwrap().base_loc;
            // Step.1 draw template
            // println!("template_path: {:?}", template_path);
            let mut z_f = archive.by_name(template_path.as_str()).unwrap();
            let mut s = String::new();
            let _ = &z_f.read_to_string(&mut s).unwrap();
            let content_page: ContentPage = quick_xml::de::from_str(s.as_str())
                .expect("Failed to parse XML");
            // println!("content_page: {:#?}", content_page);
            content_page.content.draw(backend);
        }

        //     // Step.2 draw page
        self.content.draw(backend);

        // Step.3 draw annotations
        ofd_doc.annotations.annotations.iter().for_each(|annot| {
            if annot.page_id == self.id {
                let path = format!("{}/{}", ofd_doc.annotations.dir, annot.file_loc.text);
                let mut z_f = archive.by_name(path.as_str()).unwrap();
                let mut s = String::new();
                let _ = &z_f.read_to_string(&mut s).unwrap();
                let page_annot: PageAnnot = quick_xml::de::from_str(s.as_str()).expect("Failed to parse XML");
                page_annot.draw(backend);
            }
        });

        let mut seal_file: Option<ZipArchive<Cursor<Vec<u8>>>> = archive.by_name("Doc_0/Signs/Sign_0/SignedValue.dat")
            .map_or(None, |mut file| {
            let mut buf: Vec<u8> = Vec::new();
            let _ = &file.read_to_end(&mut buf).unwrap();
            let buff = Cursor::new(buf);
            Some(ZipArchive::new(buff).unwrap())
        });
        let mut stamp = archive.by_name("Doc_0/Signs/Sign_0/Signature.xml")
            .map_or(None, |mut file| {
            let signature_ele = Element::parse(&mut file).unwrap();
            let signed_info = signature_ele.get_child("SignedInfo").unwrap();
            signed_info.get_child("StampAnnot")
                .unwrap()
                .attributes
                .get("Boundary")
                .map(|b| boundary_to_rect(b))
        });

        if seal_file.is_some() {
            let mut seal_archive = seal_file.unwrap();
            let mut z_f = seal_archive.by_name("Doc_0/PublicRes_0.xml").unwrap();
            let mut s = String::new();
            let _ = &z_f.read_to_string(&mut s).unwrap();
            let seal_pub_res: OFDRes = quick_xml::de::from_str(&s).unwrap();
            for font in &seal_pub_res.fonts {
                let family_name = font.family_name.clone().unwrap_or(font.font_name.clone());
                RES_FONT_ID_MAP.lock().unwrap().insert(
                    font.id.clone(),
                    SendWrapper::new(get_font_from_family_name(family_name.as_str())),
                );
            }
            drop(z_f);

            for j in 0..seal_archive.len() {
                let seal_f = seal_archive.by_index(j).unwrap();
                println!("seal_f: {:?}", seal_f.name());
            }
            // let seal_content = seal_archive.by_name("Doc_0/Pages/Page_0/Content.xml").unwrap();
            let seal_page = OFDPage::new(
                &mut seal_archive,
                "Doc_0/Pages/Page_0/Content.xml",
                0,
                String::from("0"),
            );
            println!("seal_page: {:?}", seal_page);
            let transform = backend.save();
            backend.draw_boundary(stamp.as_ref().unwrap());
            seal_page.content.draw(backend);
            backend.restore(&transform);
        }

        let out_f_name = format!("{}_page_{}.png", base_name, self.idx);
        backend.output_page(&out_f_name).expect("output page failed");
    }
}

impl OFDDoc {
    pub fn draw_pages(&self, archive: &mut ZipArchive<File>, doc_name: &str) {
        for page in &self.pages {
            page.draw(archive, self, doc_name);
        }
    }
}

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
        let sys_font = SystemSource::new()
            .select_best_match(
                &[FamilyName::Title(family_name.clone())],
                &Properties::new().weight(Weight::NORMAL),
            )
            .unwrap()
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PageArea {
    #[serde(deserialize_with = "deserialize_physical_box")]
    pub physical_box: PhysicalBox,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CommonData {
    #[serde(rename = "MaxUnitID")]
    pub max_unit_id: u32,
    pub template_page: Vec<TemplatePage>,
    pub page_area: Option<PageArea>,
    pub public_res: String,
    pub document_res: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TemplatePage {
    #[serde(rename = "@ID")]
    pub id: String,
    #[serde(rename = "@BaseLoc")]
    pub base_loc: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Page {
    #[serde(rename = "@ID")]
    pub id: String,
    #[serde(rename = "@BaseLoc")]
    pub base_loc: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Pages {
    pub page: Vec<Page>,
}

fn deserialize_physical_box<'de, D>(deserializer: D) -> Result<PhysicalBox, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let v: Vec<&str> = s.split_whitespace().collect();
    let x = v[0].parse::<f32>().unwrap();
    let y = v[1].parse::<f32>().unwrap();
    let width = v[2].parse::<f32>().unwrap();
    let height = v[3].parse::<f32>().unwrap();
    Ok(PhysicalBox {
        x,
        y,
        width,
        height,
    })
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OFDDocument {
    pub common_data: CommonData,
    pub pages: Pages,
    pub annotations: String,
    pub attachments: Option<String>,
    pub custom_tags: Option<String>,
}


#[cfg(test)]
mod tests {
    use crate::ofd::{CommonData, ContentPage, OFDAnnotations, OFDDocument, OFDMultiMedias, OFDPage, OFDRes, PageAnnot};
    use image::io::Reader;
    use std::io::BufReader;

    #[test]
    fn test_ofd() {
        println!("test_ofd");
        let f_name = "dzfp_23442000000075223501_20230601201823/Doc_0/Document.xml";
        let f = std::fs::File::open(f_name).unwrap();
        let ofd_document: OFDDocument =
            quick_xml::de::from_reader(BufReader::new(f)).expect("Failed to parse XML");
        print!("{:#?}", ofd_document);
    }

    #[test]
    fn test_des_ofd_document_res() {
        let f_name = "dzfp_23442000000075223501_20230601201823/Doc_0/DocumentRes.xml";
        // let f_name = "dzfp_23442000000075223501_20230601201823/Doc_0/PublicRes.xml";
        let f = std::fs::File::open(f_name).unwrap();
        let multimedias: OFDRes =
            quick_xml::de::from_reader(BufReader::new(f)).expect("Failed to parse XML");
        print!("{:#?}", multimedias);
    }

    #[test]
    fn test_des_ofd_annotations() {
        let f_name = "dzfp_23442000000075223501_20230601201823/Doc_0/Annots/Annotations.xml";
        let f = std::fs::File::open(f_name).unwrap();
        let annotations: OFDAnnotations = quick_xml::de::from_reader(BufReader::new(f))
            .expect("Failed to parse XML");
        println!("{:#?}", annotations);
    }

    #[test]
    fn test_des_ofd_pageannot() {
        let f_name = "dzfp_23442000000075223501_20230601201823/Doc_0/Annots/Page_0/Annotation.xml";
        let f = std::fs::File::open(f_name).unwrap();
        let annotations: PageAnnot = quick_xml::de::from_reader(BufReader::new(f))
            .expect("Failed to parse XML");
        println!("{:#?}", annotations);
    }

    #[test]
    fn test_ofd_content() {
        // let f_name = "dzfp_23442000000075223501_20230601201823/Doc_0/Tpls/Tpl_0/Content.xml";
        let f_name = "dzfp_23442000000075223501_20230601201823/Doc_0/Pages/Page_1/Content.xml";
        let f = std::fs::File::open(f_name).unwrap();
        // let mut ofd_page: OFDPage = quick_xml::de::from_reader(BufReader::new(f)).expect("Failed to parse XML");
        let page_content: ContentPage = quick_xml::de::from_reader(BufReader::new(f))
            .expect("Failed to parse XML");
        println!("page_content: {:#?}", page_content);
    }

    #[test]
    fn test_path_join() {
        let path = std::path::Path::new("/a/b/c");
        println!("path: {:?}", String::from(path.to_str().unwrap()) + "/d/e/f");
        let path = path.join("d/e/f");
        println!("path: {:?}", path);
    }
}
