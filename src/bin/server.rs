use std::{env, path::PathBuf, process};

const USAGE: &str = "\
Usage: server [OPTIONS]

Options:
    -a, --addr <ADDR>           address to bind
    -p, --port <PORT>           port for TCP and UDP
    -o, --out <DIR>             output directory
    -b, --buffer-size <BYTES>   buffer size, bytes
    -h, --help                  show this help
";

fn usage_exit() -> ! {
    eprintln!("{USAGE}");
    process::exit(2);
}

fn arg_value(args: &mut impl Iterator<Item = String>) -> String {
    args.next().unwrap_or_else(|| usage_exit())
}

fn parse_args() -> Config {
    let mut cfg = Config {
        host: "0.0.0.0".to_string(),
        port: 666,
        out: PathBuf::from("received"),
        buffer: 64 * 1024,
    };

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-a" | "--addr" => cfg.host = arg_value(&mut args),
            "-h" | "--help" => {
                println!("{USAGE}");
                process::exit(0);
            }
            _ => usage_exit(),
        }
    }
    cfg
}

struct Config {
    host: String,
    port: u16,
    out: PathBuf,
    buffer: usize,
}

fn main() {
    println!("Hello, world!");
    let cfg = parse_args();
    println!("{}", cfg.host);
}
