//! Limpeza de metadado de imagem, sem mexer nos pixels.
//!
//! WebP e PNG são listas de blocos (chunks). A limpeza mantém só os blocos
//! que desenham a imagem e joga fora o resto (EXIF, XMP, texto, manifesto de procedência e
//! qualquer bloco desconhecido). Os bytes dos pixels saem idênticos: a
//! imagem não é recodificada nem cortada.

/// Blocos de WebP que ficam (o resto sai).
const WEBP_KEEP: &[&[u8; 4]] = &[
    b"VP8 ", b"VP8L", b"VP8X", b"ALPH", b"ANIM", b"ANMF", b"ICCP",
];

/// Blocos de PNG que ficam (o resto sai, inclusive tEXt, zTXt, iTXt, eXIf,
/// tIME e caBX, o bloco do manifesto de procedência).
const PNG_KEEP: &[&[u8; 4]] = &[
    b"IHDR", b"PLTE", b"IDAT", b"IEND", b"tRNS", b"gAMA", b"cHRM", b"sRGB", b"iCCP", b"sBIT",
    b"pHYs", b"bKGD", b"acTL", b"fcTL", b"fdAT",
];

/// Blocos que só carregam pixels; a varredura de texto não olha dentro deles.
const PIXEL_CHUNKS: &[&[u8; 4]] = &[b"VP8 ", b"VP8L", b"ALPH", b"ANMF", b"IDAT", b"fdAT"];

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Webp,
    Png,
}

pub fn kind_of(path: &std::path::Path) -> Option<Kind> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "webp" => Some(Kind::Webp),
        "png" => Some(Kind::Png),
        _ => None,
    }
}

/// Um bloco: tipo e conteúdo.
struct Chunk<'a> {
    tag: [u8; 4],
    data: &'a [u8],
}

fn webp_chunks(bytes: &[u8]) -> Result<Vec<Chunk<'_>>, String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return Err("não é um WebP (cabeçalho RIFF/WEBP ausente)".into());
    }
    let mut chunks = Vec::new();
    let mut pos = 12;
    while pos < bytes.len() {
        if pos + 8 > bytes.len() {
            return Err("WebP truncado no cabeçalho de um bloco".into());
        }
        let tag: [u8; 4] = bytes[pos..pos + 4].try_into().expect("4 bytes");
        let size =
            u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().expect("4 bytes")) as usize;
        let start = pos + 8;
        let end = start
            .checked_add(size)
            .filter(|&e| e <= bytes.len())
            .ok_or("WebP truncado dentro de um bloco")?;
        chunks.push(Chunk {
            tag,
            data: &bytes[start..end],
        });
        pos = end + (size & 1);
    }
    Ok(chunks)
}

fn png_chunks(bytes: &[u8]) -> Result<Vec<Chunk<'_>>, String> {
    if bytes.len() < 8 || &bytes[0..8] != PNG_SIGNATURE {
        return Err("não é um PNG (assinatura ausente)".into());
    }
    let mut chunks = Vec::new();
    let mut pos = 8;
    while pos < bytes.len() {
        if pos + 12 > bytes.len() {
            return Err("PNG truncado no cabeçalho de um bloco".into());
        }
        let size = u32::from_be_bytes(bytes[pos..pos + 4].try_into().expect("4 bytes")) as usize;
        let tag: [u8; 4] = bytes[pos + 4..pos + 8].try_into().expect("4 bytes");
        let start = pos + 8;
        let end = start
            .checked_add(size)
            .filter(|&e| e + 4 <= bytes.len())
            .ok_or("PNG truncado dentro de um bloco")?;
        chunks.push(Chunk {
            tag,
            data: &bytes[start..end],
        });
        pos = end + 4; // pula o CRC
    }
    Ok(chunks)
}

fn chunks_of(kind: Kind, bytes: &[u8]) -> Result<Vec<Chunk<'_>>, String> {
    match kind {
        Kind::Webp => webp_chunks(bytes),
        Kind::Png => png_chunks(bytes),
    }
}

fn keep_list(kind: Kind) -> &'static [&'static [u8; 4]] {
    match kind {
        Kind::Webp => WEBP_KEEP,
        Kind::Png => PNG_KEEP,
    }
}

/// Devolve a imagem sem os blocos de metadado. Os blocos mantidos saem com
/// os mesmos bytes, na mesma ordem.
pub fn strip(kind: Kind, bytes: &[u8]) -> Result<Vec<u8>, String> {
    let keep = keep_list(kind);
    let chunks = chunks_of(kind, bytes)?;
    let mut out = Vec::with_capacity(bytes.len());
    match kind {
        Kind::Webp => {
            out.extend_from_slice(b"RIFF\0\0\0\0WEBP");
            for c in chunks.iter().filter(|c| keep.contains(&&c.tag)) {
                out.extend_from_slice(&c.tag);
                out.extend_from_slice(&(c.data.len() as u32).to_le_bytes());
                if &c.tag == b"VP8X" && !c.data.is_empty() {
                    // Desliga os avisos de EXIF (bit 3) e XMP (bit 2): os blocos saíram.
                    out.push(c.data[0] & !0b0000_1100);
                    out.extend_from_slice(&c.data[1..]);
                } else {
                    out.extend_from_slice(c.data);
                }
                if c.data.len() % 2 == 1 {
                    out.push(0);
                }
            }
            let riff_size = (out.len() - 8) as u32;
            out[4..8].copy_from_slice(&riff_size.to_le_bytes());
        }
        Kind::Png => {
            out.extend_from_slice(PNG_SIGNATURE);
            let mut pos = 8;
            // Copia cada bloco mantido do original, com o CRC original.
            while pos < bytes.len() {
                let size =
                    u32::from_be_bytes(bytes[pos..pos + 4].try_into().expect("4 bytes")) as usize;
                let tag: [u8; 4] = bytes[pos + 4..pos + 8].try_into().expect("4 bytes");
                let end = pos + 12 + size;
                if keep.contains(&&tag) {
                    out.extend_from_slice(&bytes[pos..end]);
                }
                pos = end;
            }
        }
    }
    Ok(out)
}

/// Confere que a imagem só tem blocos permitidos e devolve o conteúdo dos
/// blocos que não são pixel, para a varredura de texto.
pub fn non_pixel_bytes(kind: Kind, bytes: &[u8]) -> Result<Vec<u8>, String> {
    let keep = keep_list(kind);
    let mut out = Vec::new();
    for c in chunks_of(kind, bytes)? {
        if !keep.contains(&&c.tag) {
            return Err(format!(
                "bloco de metadado {:?} presente (rode a limpeza)",
                String::from_utf8_lossy(&c.tag)
            ));
        }
        if !PIXEL_CHUNKS.contains(&&c.tag) {
            out.extend_from_slice(&c.tag);
            out.extend_from_slice(c.data);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn webp_with(chunks: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
        let mut v = b"RIFF\0\0\0\0WEBP".to_vec();
        for (tag, data) in chunks {
            v.extend_from_slice(*tag);
            v.extend_from_slice(&(data.len() as u32).to_le_bytes());
            v.extend_from_slice(data);
            if data.len() % 2 == 1 {
                v.push(0);
            }
        }
        let n = (v.len() - 8) as u32;
        v[4..8].copy_from_slice(&n.to_le_bytes());
        v
    }

    #[test]
    fn webp_strip_keeps_pixels_and_drops_metadata() {
        let original = webp_with(&[
            (b"VP8X", &[0b0000_1100, 0, 0, 0, 1, 0, 0, 1, 0, 0]),
            (b"VP8 ", b"pixels!"),
            (b"EXIF", b"camera"),
            (b"XMP ", b"<x:xmpmeta/>"),
            // O tag montado byte a byte não vira achado da própria varredura.
            #[allow(clippy::byte_char_slices)]
            (&[b'C', b'2', b'P', b'A'], b"manifest"),
        ]);
        let clean = strip(Kind::Webp, &original).unwrap();
        let expected = webp_with(&[
            (b"VP8X", &[0, 0, 0, 0, 1, 0, 0, 1, 0, 0]),
            (b"VP8 ", b"pixels!"),
        ]);
        assert_eq!(clean, expected);
        assert!(non_pixel_bytes(Kind::Webp, &clean).is_ok());
        assert!(non_pixel_bytes(Kind::Webp, &original).is_err());
    }

    #[test]
    fn png_strip_drops_text_and_provenance() {
        fn chunk(tag: &[u8; 4], data: &[u8]) -> Vec<u8> {
            let mut v = (data.len() as u32).to_be_bytes().to_vec();
            v.extend_from_slice(tag);
            v.extend_from_slice(data);
            v.extend_from_slice(&[1, 2, 3, 4]); // CRC fictício: a limpeza não o recalcula
            v
        }
        let ihdr = chunk(b"IHDR", &[0; 13]);
        let idat = chunk(b"IDAT", b"zzz");
        let iend = chunk(b"IEND", &[]);
        let mut original = PNG_SIGNATURE.to_vec();
        for c in [
            &ihdr,
            &chunk(b"tEXt", b"Software\0x"),
            &chunk(b"caBX", b"jumb"),
            &idat,
            &iend,
        ] {
            original.extend_from_slice(c);
        }
        let mut expected = PNG_SIGNATURE.to_vec();
        for c in [&ihdr, &idat, &iend] {
            expected.extend_from_slice(c);
        }
        assert_eq!(strip(Kind::Png, &original).unwrap(), expected);
    }
}
