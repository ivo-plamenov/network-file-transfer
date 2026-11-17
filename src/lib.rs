use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

pub use sha2::{Digest, Sha256};

pub const PACKET_MARKER: &[u8; 4] = b"PM01";
pub const DIGEST_LENGTH: usize = 32;

pub type Sha256Digest = [u8; DIGEST_LENGTH];

// TCP
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TcpHeader {
    pub name: String,
    pub size: u64,
    pub sha256: Sha256Digest,
}

pub fn write_tcp_header<W: Write>(w: &mut W, h: TcpHeader) -> io::Result<()> {
    let name_bytes = h.name.as_bytes();
    let len = u16::try_from(name_bytes.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "File name too long"))?;

    let mut buffer = Vec::with_capacity(4 + 2 + name_bytes.len() + 8 + DIGEST_LENGTH);
    buffer.extend_from_slice(PACKET_MARKER);
    buffer.extend_from_slice(&len.to_be_bytes());
    buffer.extend_from_slice(name_bytes);
    buffer.extend_from_slice(&h.size.to_be_bytes());
    buffer.extend_from_slice(&h.sha256);

    w.write_all(&buffer)
}

pub fn read_tcp_header<R: Read>(r: &mut R) -> io::Result<TcpHeader> {
    let mut packet_marker = [0u8; 4];
    r.read_exact(&mut packet_marker)?;

    if &packet_marker != PACKET_MARKER {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Bad packet marker",
        ));
    }

    let mut len = [0u8, 2];
    r.read_exact(&mut len)?;
    let mut name = vec![0u8; u16::from_be_bytes(len) as usize];
    r.read_exact(&mut name)?;
    let name = String::from_utf8(name)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "File name is not UTF-8"))?;
    let mut size = [0u8; 8];
    r.read_exact(&mut size)?;
    let mut sha256 = [0u8; DIGEST_LENGTH];
    r.read_exact(&mut sha256)?;

    Ok(TcpHeader {
        name,
        size: u64::from_be_bytes(size),
        sha256,
    })
}

pub fn get_filename(file_path: &str) -> String {
    Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.is_empty())
        .unwrap_or("unknown-name")
        .to_string()
}

pub fn calculate_file_hash(path: &Path, buffer_size: usize) -> io::Result<Sha256Digest> {
    let mut file = File::open(path)?;
    let mut buffer = vec![0u8; buffer_size.max(1)];
    let mut hasher = Sha256::new();

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(finish(hasher))
}

pub fn finish(hasher: Sha256) -> Sha256Digest {
    let mut out = [0u8; DIGEST_LENGTH];
    out.copy_from_slice(&hasher.finalize());

    out
}

pub fn to_hex(digest: &Sha256Digest) -> String {
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn calculate_mb_per_sec(bytes: u64, secs: f64) -> f64 {
    if secs > 0.0 {
        bytes as f64 / 1_000_000.0 / secs
    } else {
        0.0
    }
}
