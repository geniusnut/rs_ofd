mod scrawl;
mod utils;
mod ofd;

use std::fs;
use crate::ofd::OFDFile;
use crate::utils::node_draw::draw_path;

use jbig2dec::Document;

fn main() {
    std::process::exit(real_main());
}

fn real_main() -> i32 {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <filename>", args[0]);
        return 1;
    }
    let fname = std::path::Path::new(&*args[1]);
    let file = fs::File::open(&fname).unwrap();

    let mut ofd_file =  OFDFile { ..Default::default() };
    println!("{:?}", ofd_file);
    0
}
