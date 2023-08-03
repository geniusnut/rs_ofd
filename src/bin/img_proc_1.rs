use std::env;

use image::{GenericImageView, Rgb, RgbImage};
use imageproc::drawing::draw_antialiased_line_segment_mut;
use imageproc::pixelops::interpolate;

use std::path::Path;
use jbig2dec::Document;

fn main() {
    // let arg = if env::args().count() == 2 {
    //     env::args().nth(1).unwrap()
    // } else {
    //     panic!("Please enter a target file path")
    // };
    //
    // let path = Path::new(&arg);
    // let red = Rgb([255u8, 0u8, 0u8]);
    //
    // let mut image = RgbImage::new(1587, 1058);
    //
    // draw_antialiased_line_segment_mut(&mut image, (20, 12), (1000, 1000), red, interpolate);
    //
    // image.save(path).unwrap();

    // Use the open function to load an image from a Path.
    // `open` returns a `DynamicImage` on success.
    let doc = Document::open("1638367527374/Doc_0/Res/image_79.jb2").expect("open document failed");
    for image in doc.into_iter() {
        println!("image: len {}", image.data().len());
        let img = image.to_png().unwrap();
        let dyn_image = image::load_from_memory(&img).expect("convert to DynamicImage failed");

        // The dimensions method returns the images width and height.
        println!("dimensions {:?}, data_size: {}", dyn_image.dimensions(), dyn_image.as_bytes().len());

        // The color method returns the image's `ColorType`.
        println!("{:?}", dyn_image.color());

        // Write the contents of this image to the Writer in PNG format.
        dyn_image.save("image_81.png").unwrap();
    }
}
