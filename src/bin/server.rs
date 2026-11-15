use std::{env, fs, net::TcpListener, path::PathBuf, process};

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
            "-p" | "--port" => {
                cfg.port = arg_value(&mut args)
                    .parse()
                    .unwrap_or_else(|_| usage_exit())
            }
            "-o" | "--out" => cfg.out = PathBuf::from(arg_value(&mut args)),
            "-b" | "--buffer-size" => {
                cfg.buffer = arg_value(&mut args)
                    .parse()
                    .unwrap_or_else(|_| usage_exit());
                if cfg.buffer == 0 {
                    usage_exit();
                }
            }
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
    let cfg = parse_args();
    let socket_bind = format!("{}:{}", cfg.host, cfg.port);

    fs::create_dir_all(&cfg.out).unwrap_or_else(|e| {
        eprintln!("Cannot create {}: {e}", cfg.out.display());
        process::exit(1);
    });

    let listener = TcpListener::bind(&socket_bind).unwrap_or_else(|e| {
        eprintln!("Cannot bind TCP to {socket_bind}: {e}");
        process::exit(1);
    });
    println!(
        "Listening on {socket_bind}, saving to: {}; buffer: {} bytes",
        cfg.out.display(),
        cfg.buffer
    );
}
