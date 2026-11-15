use std::io::{self, Read, Write};

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
