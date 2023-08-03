use zip;
use xmltree::Element;
use zip::read::ZipFile;
use std::default::{Default};
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;
use jbig2dec::Document;
use raqote::{DrawOptions, DrawTarget, SolidSource, Source, Transform, Vector};

use image_demo::ofd::{OFD_XML, OFDFile, OFDDoc, PhysicalBox};
use image_demo::utils::node_draw::{boundary_to_rect, draw_ele_recursive, MUTEX_FONT_RES, PPMM, get_font_from_family_name, MUTEX_IMAGE_RES, MUTEX_JBIG_RES};


#[allow(dead_code)]
fn indent(size: usize) -> String {
    const INDENT: &'static str = "    ";
    (0..size)
        .map(|_| INDENT)
        .fold(String::with_capacity(size * INDENT.len()), |r, s| r + s)
}

fn read_xml_element(zf: &mut ZipFile) -> Element {
    let ofd_element = Element::parse(zf).unwrap();
    ofd_element.clone()
}

#[allow(dead_code)]
fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}

fn main() {
    std::process::exit(real_main());
}

fn real_main() -> i32 {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <filename>", args[0]);
        return 1;
    }
    let fname = Path::new(&*args[1]);
    let out_f_name = fname.with_extension("").with_extension("png");

    let file = File::open(&fname).unwrap();

    let ofd_file: &mut OFDFile = &mut OFDFile::default();
    let mut doc_body : Option<Element> = None;
    let mut archive = zip::ZipArchive::new(file).unwrap();
    for i in 0..archive.len() {
        let file = archive.by_index(i).unwrap();
        if (*file.name()).eq(OFD_XML) {
            let ofd_element = Element::parse(file).unwrap();

            doc_body = Some(ofd_element.children[0].as_element().unwrap().clone());
            break;
        }
    }

    let doc_body_v = doc_body.unwrap();
    let info = doc_body_v.get_child("DocInfo");
    println!("DocInfo: {:?}", info.unwrap());

    let doc_root = String::from(doc_body_v.get_child("DocRoot").unwrap().get_text().unwrap());
    // println!("DocRoot: {:?}", doc_root);
    ofd_file.doc_root = Path::new(&doc_root).parent().unwrap().to_str().unwrap().to_string();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let doc_root_path = Path::new(&doc_root).parent().unwrap();
        if (*file.name()).eq(&doc_root) {
            let doc_node = read_xml_element(&mut file);
            // println!("doc_0_node: {:?}", doc_0_node);
            let common_data = doc_node.get_child("CommonData").unwrap();
            let template_page = common_data.get_child("TemplatePage").unwrap().attributes.get("BaseLoc");
            let physical_box = common_data.get_child("PageArea").unwrap().get_child("PhysicalBox").unwrap();
            let public_res =  String::from(common_data.get_child("PublicRes").unwrap().get_text().unwrap());
            let document_res = String::from(common_data.get_child("DocumentRes").unwrap().get_text().unwrap());
            let p_box = boundary_to_rect(&String::from(physical_box.get_text().unwrap()));
            let annotations = String::from(doc_node.get_child("Annotations").unwrap().get_text().unwrap());
            let attachment = doc_node.get_child("Attachments").map_or(String::new(), |e| String::from(e.get_text().unwrap()));

            ofd_file.ofd_doc = Some(OFDDoc {
                template_page: Some(template_page.unwrap().clone()),
                physical_box: Some(p_box),
                document_res: doc_root_path.join(Path::new(&document_res)).to_str().map(|s|String::from(s)),
                public_res:  doc_root_path.join(Path::new(&public_res)).to_str().map(|s|String::from(s)),
                pages: vec![],
                annotations: Some(annotations),
                attachment: Some(attachment),
            });
        }
    }

    let ofd_doc = ofd_file.ofd_doc.clone().unwrap();
    println!("ofd_doc: {:?}", ofd_doc);
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();

        if (*file.name()).eq(&ofd_doc.annotations.clone().unwrap_or_default()) {
        } else if (*file.name()).eq(&ofd_doc.public_res.clone().unwrap_or_default()) {
            let res_node = read_xml_element(&mut file);
            let fonts = res_node.get_child("Fonts").unwrap().clone();
            for font_node in fonts.children {
                let font_ele = font_node.as_element().unwrap().clone();
                let id = font_ele.attributes.get("ID").unwrap().clone();
                let family_name = font_ele.attributes.get("FamilyName").unwrap().clone();
                // let a = MutexFontRes.lock().unwrap().get("").unwrap();
                // let candidates = FONT_FAMILIY_NAME_MAP.get(family_name.as_str()).map_or([FamilyName::Title(family_name)],
                //     |l| l.iter().map(|e| FamilyName::Title(e)).collect());
                let font = get_font_from_family_name(family_name.as_str());
                MUTEX_FONT_RES.lock().unwrap().insert(id, font);
            }
        } else if (*file.name()).eq(&ofd_doc.document_res.clone().unwrap_or_default()) {
            let res_node = read_xml_element(&mut file);
            let multimedias = res_node.get_child("MultiMedias").unwrap().clone();
            for mm_node in multimedias.children {
                let mm_ele = mm_node.as_element().unwrap().clone();
                let mm_id = mm_ele.attributes.get("ID").unwrap().clone();
                let mm_type = mm_ele.attributes.get("Type").unwrap().clone();
                if mm_type.eq("Image") {
                    let media_file = mm_ele.get_child("MediaFile").unwrap().get_text().unwrap().clone();
                    println!("media_file: {:?}", media_file);
                    MUTEX_IMAGE_RES.lock().unwrap().insert(mm_id, String::from(media_file.as_ref()));
                }
            }
        }
    }

    // print_type_of(&MUTEX_IMAGE_RES.lock().unwrap().values());

    // get image bytes.
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        for v in MUTEX_IMAGE_RES.lock().unwrap().values() {
            if (*file.name()).ends_with(v) {
                let mut buf: Vec<u8> = Vec::new();
                let _ = &file.read_to_end(&mut buf).unwrap();
                let mut buff = Cursor::new(buf);
                let doc = Document::from_reader(&mut buff).expect("");
                let image = doc.images().get(0).unwrap().clone();
                let img = image.to_png().unwrap();
                let dyn_image = image::load_from_memory(&img).expect("convert to DynamicImage failed").into_rgba8();
                MUTEX_JBIG_RES.lock().unwrap().insert(String::from(v), dyn_image);
            }
        }
    }
    // println!("MUTEX_FONT_RES: {:?}", MUTEX_FONT_RES.lock().unwrap());
    let mut p_box = ofd_doc.physical_box.unwrap();
    p_box.width = p_box.width * PPMM;
    p_box.height = p_box.height * PPMM;
    println!("p_box: {:?}", p_box);

    let mut dt = DrawTarget::new(p_box.width as i32, p_box.height as i32);
    dt.fill_rect(0., 0., p_box.width, p_box.height, &Source::Solid( SolidSource{r: 0xff,
        g: 0xff,
        b: 0xff,
        a: 0xff,}), &DrawOptions::new());
    dt.set_transform(&Transform::scale(PPMM, PPMM));
    // println!("transform {:?}",  dt.get_transform());

    let mut seal_file: Option<zip::ZipArchive<Cursor<Vec<u8>>>> = None;
    let mut stamp: Option<PhysicalBox> = None;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        if (*file.name()).eq("Doc_0/Tpls/Tpl_0/Content.xml") {
            let tpl_ele = read_xml_element(&mut file);
            let tpl_content = tpl_ele.get_child("Content").unwrap();
            draw_ele_recursive(&mut dt, &tpl_content);
        } else if (*file.name()).eq("Doc_0/Pages/Page_0/Content.xml"){
            let content_ele = read_xml_element(&mut file);
            draw_ele_recursive(&mut dt, &content_ele);
        } else if (*file.name()).eq("Doc_0/Signs/Sign_0/SignedValue.dat") {
            let mut buf: Vec<u8> = Vec::new();
            let _ = &file.read_to_end(&mut buf).unwrap();
            let buff = Cursor::new(buf);
            seal_file = Some(zip::ZipArchive::new(buff).unwrap());
        } else if (*file.name()).eq("Doc_0/Signs/Sign_0/Signature.xml") {
            let signature_ele = Element::parse(&mut file).unwrap();
            let signed_info = signature_ele.get_child("SignedInfo").unwrap();
            stamp = Some(signed_info.get_child("StampAnnot").unwrap().attributes.get("Boundary")
                .map(|b| boundary_to_rect(b)).unwrap_or(PhysicalBox::default()));
        }
    }

    // draw seal
    if seal_file.is_some() {
        let mut sign_archive = seal_file.unwrap();
        // 0. extract seal resource
        for j in 0..sign_archive.len() {
            let seal_f = sign_archive.by_index(j).unwrap();
            if (*seal_f.name()).eq("Doc_0/PublicRes_0.xml") {
                let seal_element = Element::parse(seal_f).unwrap();
                let fonts = seal_element.get_child("Fonts").unwrap().clone();
                for font_node in fonts.children {
                    let font_ele = font_node.as_element().unwrap().clone();
                    let id = font_ele.attributes.get("ID").unwrap().clone();
                    let family_name = font_ele.attributes.get("FontName").unwrap().clone();
                    // let a = MutexFontRes.lock().unwrap().get("").unwrap();
                    // let candidates = FONT_FAMILIY_NAME_MAP.get(family_name.as_str()).map_or([FamilyName::Title(family_name)],
                    //     |l| l.iter().map(|e| FamilyName::Title(e)).collect());
                    let font = get_font_from_family_name(family_name.as_str());
                    MUTEX_FONT_RES.lock().unwrap().insert(id, font);
                }
            }
        }
        // println!("MUTEX_FONT_RES: {:?}", MUTEX_FONT_RES.lock().unwrap());

        for j in 0..sign_archive.len() {
            let seal_f = sign_archive.by_index(j).unwrap();
            if (*seal_f.name()).eq("Doc_0/Pages/Page_0/Content.xml") {
                let seal_element = Element::parse(seal_f).unwrap();
                // println!("seal_element: {:?}", seal_element);
                let m = dt.get_transform().clone();
                if stamp.is_some() {
                    let b = stamp.unwrap();
                    let mm = m.pre_translate(Vector::new(b.x, b.y));
                    dt.set_transform(&mm);
                }
                draw_ele_recursive(&mut dt, &seal_element);
                dt.set_transform(&m);
                break;
            }
        }
    }

    dt.write_png(out_f_name).expect("write png file failed");
    0
}

