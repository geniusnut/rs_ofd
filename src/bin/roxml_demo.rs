use image_demo::ofd::OFD_XML;
use roxmltree::{Document, Node};
use std::borrow::Borrow;
use std::fs;
use std::io::Read;

fn main() {
    std::process::exit(real_main());
}

struct OFDDoc1 {}

fn real_main() -> i32 {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <filename>", args[0]);
        return 1;
    }
    let mut ofd_body: Option<Node> = None;
    let fname = std::path::Path::new(&*args[1]);
    let file = fs::File::open(&fname).unwrap();
    let mut buf = String::new();

    let mut archive = zip::ZipArchive::new(file).unwrap();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        if (*file.name()).eq(OFD_XML) {
            file.read_to_string(&mut buf).expect("read failed");
        }
    }

    let ofd_doc = Document::parse(buf.as_str()).unwrap();
    let ofd_c = ofd_doc.root_element().first_child().unwrap().clone();
    ofd_body = Some(ofd_c);

    for node in ofd_body.unwrap().children() {
        println!("node: {:?}", node.tag_name());
    }
    0
}
