use image_demo::ofd::OFDFile;

#[allow(dead_code)]
fn indent(size: usize) -> String {
    const INDENT: &'static str = "    ";
    (0..size)
        .map(|_| INDENT)
        .fold(String::with_capacity(size * INDENT.len()), |r, s| r + s)
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
        println!("Usage: {} <filename.ofd> ...", args[0]);
        return 1;
    }
    args.iter().skip_while(|arg| {
        !arg.ends_with(".ofd")
    }).for_each(|arg| {
        println!("Input ofd: {}", arg);
        let mut ofd_file = OFDFile::new(arg);
        ofd_file.draw();
    });
    0
}
