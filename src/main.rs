use std::path::PathBuf;

struct Config {
    host: String,
    port: u16,
    out: PathBuf,
    buffer: usize,
}

fn main() {
    println!("Hello, world!");
}
