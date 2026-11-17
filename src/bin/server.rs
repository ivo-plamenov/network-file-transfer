use network_file_transfer::{
    calculate_file_hash, calculate_mb_per_sec, get_filename, read_tcp_header, to_hex,
};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use std::{env, process, thread};

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

fn output_path(out_dir: &Path, name: &str) -> PathBuf {
    out_dir.join(format!("received_{}", get_filename(name)))
}

static ACTIVE_CONNECTIONS: AtomicUsize = AtomicUsize::new(0);

struct ActiveConnection;

impl ActiveConnection {
    fn enter() -> (ActiveConnection, usize) {
        let now = ACTIVE_CONNECTIONS.fetch_add(1, Ordering::SeqCst) + 1;
        (ActiveConnection, now)
    }
}

impl Drop for ActiveConnection {
    fn drop(&mut self) {
        ACTIVE_CONNECTIONS.fetch_sub(1, Ordering::SeqCst);
    }
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

    let tcp_listener = TcpListener::bind(&socket_bind).unwrap_or_else(|e| {
        eprintln!("Cannot bind TCP to {socket_bind}: {e}");
        process::exit(1);
    });
    println!(
        "Listening on {socket_bind}, saving to: {}; buffer: {} bytes",
        cfg.out.display(),
        cfg.buffer
    );

    for tcp_stream in tcp_listener.incoming() {
        match tcp_stream {
            Ok(tcp_stream) => {
                let out = cfg.out.clone();
                let buffer = cfg.buffer;
                thread::spawn(move || {
                    let peer = tcp_stream
                        .peer_addr()
                        .map(|addr| addr.to_string())
                        .unwrap_or_else(|_| "?".to_string());
                    let (_connection, active) = ActiveConnection::enter();

                    println!("[tcp] {peer} connected (active connections: {active})");

                    if let Err(e) = handle_tcp(tcp_stream, &peer, &out, buffer) {
                        eprintln!("[tcp] {peer}: error: {e}");
                    }
                });
            }
            Err(e) => eprint!("[tcp] accept failed: {e}"),
        }
    }
}

fn handle_tcp(
    mut stream: TcpStream,
    peer: &str,
    out_dir: &Path,
    buf_size: usize,
) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;

    let header = read_tcp_header(&mut stream)?;
    let path = output_path(out_dir, &header.name);
    let mut file = File::create(&path)?;

    let start = Instant::now();
    let mut buf = vec![0u8; buf_size];
    let mut received: u64 = 0;

    while received < header.size {
        let want = (header.size - received).min(buf.len() as u64) as usize;
        let n = stream.read(&mut buf[..want])?;
        if n == 0 {
            drop(file);
            let _ = fs::remove_file(&path);
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!(
                    "The client closed after {received} of {} bytes.",
                    header.size
                ),
            ));
        }
        file.write_all(&buf[..n])?;
        received += n as u64;
    }
    let receive_secs = start.elapsed().as_secs_f64();
    drop(file);

    let actual = calculate_file_hash(&path, buf_size)?;
    let ok = actual == header.sha256;

    let reply = format!("{} {received}\n", if ok { "OK" } else { "MISMATCH" });
    stream.write_all(reply.as_bytes())?;
    stream.flush()?;

    println!(
        "[TCP] {peer} {} -> {}: received {received} bytes in {receive_secs:.3}s ({:.2} MB/s), sha256 {}",
        header.name,
        path.display(),
        calculate_mb_per_sec(received, receive_secs),
        if ok {
            "OK".to_string()
        } else {
            format!(
                "MISMATCH (exected {}, got {})",
                to_hex(&header.sha256),
                to_hex(&actual)
            )
        }
    );

    Ok(())
}
