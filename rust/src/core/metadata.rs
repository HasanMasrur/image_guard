//! Lossless metadata removal for files that are returned without re-encoding.
//! Pixels are never touched; only metadata segments/chunks are dropped.
//! Both functions return `None` when the file layout is not what they expect,
//! so the caller can fall back to a full re-encode.

/// Drops APP1 (EXIF, XMP), APP13 (IPTC/Photoshop) and COM segments.
/// Keeps everything needed for display (APP0 JFIF, APP2 ICC profile, APP14 Adobe, tables).
pub fn strip_jpeg(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len());
    out.extend_from_slice(&bytes[..2]);
    let mut i = 2;
    loop {
        if i >= bytes.len() || bytes[i] != 0xFF {
            return None;
        }
        // Skip fill bytes.
        while i < bytes.len() && bytes[i] == 0xFF {
            i += 1;
        }
        let marker = *bytes.get(i)?;
        i += 1;
        match marker {
            // Start of scan: the rest is entropy-coded data, copy verbatim.
            0xDA => {
                out.extend_from_slice(&[0xFF, 0xDA]);
                out.extend_from_slice(&bytes[i..]);
                return Some(out);
            }
            0xD9 => {
                out.extend_from_slice(&[0xFF, 0xD9]);
                return Some(out);
            }
            0x01 | 0xD0..=0xD7 => out.extend_from_slice(&[0xFF, marker]),
            _ => {
                let len = u16::from_be_bytes([*bytes.get(i)?, *bytes.get(i + 1)?]) as usize;
                if len < 2 || i + len > bytes.len() {
                    return None;
                }
                let drop = matches!(marker, 0xE1 | 0xED | 0xFE);
                if !drop {
                    out.extend_from_slice(&[0xFF, marker]);
                    out.extend_from_slice(&bytes[i..i + len]);
                }
                i += len;
            }
        }
    }
}

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// Drops text (tEXt, zTXt, iTXt), EXIF (eXIf) and timestamp (tIME) chunks.
pub fn strip_png(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.len() < 8 || bytes[..8] != PNG_SIGNATURE {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len());
    out.extend_from_slice(&PNG_SIGNATURE);
    let mut i = 8;
    while i + 12 <= bytes.len() {
        let len = u32::from_be_bytes(bytes[i..i + 4].try_into().ok()?) as usize;
        let end = i.checked_add(12)?.checked_add(len)?;
        if end > bytes.len() {
            return None;
        }
        let kind = &bytes[i + 4..i + 8];
        if !matches!(kind, b"tEXt" | b"zTXt" | b"iTXt" | b"eXIf" | b"tIME") {
            out.extend_from_slice(&bytes[i..end]);
        }
        i = end;
        if kind == b"IEND" {
            return Some(out);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
        let mut v = vec![0xFF, marker];
        v.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        v.extend_from_slice(payload);
        v
    }

    #[test]
    fn jpeg_drops_exif_and_comment_keeps_icc() {
        let mut jpg = vec![0xFF, 0xD8];
        jpg.extend(segment(0xE0, b"JFIF\0"));
        jpg.extend(segment(0xE1, b"Exif\0\0GPS-SECRET"));
        jpg.extend(segment(0xE2, b"ICC_PROFILE\0"));
        jpg.extend(segment(0xFE, b"a comment"));
        jpg.extend([0xFF, 0xDA, 1, 2, 3, 0xFF, 0xD9]);
        let out = strip_jpeg(&jpg).unwrap();
        let s = String::from_utf8_lossy(&out);
        assert!(!s.contains("GPS-SECRET"));
        assert!(!s.contains("a comment"));
        assert!(s.contains("ICC_PROFILE"));
        assert!(s.contains("JFIF"));
        assert!(out.ends_with(&[0xFF, 0xDA, 1, 2, 3, 0xFF, 0xD9]));
    }

    #[test]
    fn jpeg_truncated_segment_is_none() {
        let mut jpg = vec![0xFF, 0xD8, 0xFF, 0xE1, 0x10, 0x00];
        jpg.extend([0u8; 4]);
        assert!(strip_jpeg(&jpg).is_none());
        assert!(strip_jpeg(b"not a jpeg").is_none());
    }

    fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut v = (data.len() as u32).to_be_bytes().to_vec();
        v.extend_from_slice(kind);
        v.extend_from_slice(data);
        v.extend_from_slice(&[0, 0, 0, 0]); // CRC is copied, not checked
        v
    }

    #[test]
    fn png_drops_text_chunks() {
        let mut png = PNG_SIGNATURE.to_vec();
        png.extend(chunk(b"IHDR", &[0; 13]));
        png.extend(chunk(b"tEXt", b"Author\0secret"));
        png.extend(chunk(b"IDAT", &[1, 2, 3]));
        png.extend(chunk(b"IEND", &[]));
        let out = strip_png(&png).unwrap();
        assert!(!String::from_utf8_lossy(&out).contains("secret"));
        assert_eq!(out.len(), png.len() - (12 + 13));
    }

    #[test]
    fn png_without_iend_is_none() {
        let mut png = PNG_SIGNATURE.to_vec();
        png.extend(chunk(b"IHDR", &[0; 13]));
        assert!(strip_png(&png).is_none());
    }
}
