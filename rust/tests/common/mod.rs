#![allow(dead_code)]
//! Synthetic test images. Generated in code so tests need no binary fixtures.

use image::{DynamicImage, ImageEncoder, Rgb, RgbImage, Rgba, RgbaImage};
use jpeg_encoder::{ColorType, Encoder};

/// Looks like a photo: smooth gradients + mild noise (compresses like a real picture).
pub fn photo(w: u32, h: u32) -> RgbImage {
    let mut s = 0x9E37_79B9u32;
    RgbImage::from_fn(w, h, |x, y| {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        let n = (s % 24) as i32 - 12;
        let fx = x as f32 / w as f32;
        let fy = y as f32 / h as f32;
        let r = (255.0 * fx) as i32 + n;
        let g = (255.0 * fy) as i32 + n;
        let b = (128.0 + 127.0 * ((fx * 12.0).sin() * (fy * 9.0).cos())) as i32 + n;
        Rgb([
            r.clamp(0, 255) as u8,
            g.clamp(0, 255) as u8,
            b.clamp(0, 255) as u8,
        ])
    })
}

/// Pure noise: worst case for compression.
pub fn noise(w: u32, h: u32) -> RgbImage {
    let mut s = 0x1234_5678u32;
    RgbImage::from_fn(w, h, |_, _| {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        Rgb([s as u8, (s >> 8) as u8, (s >> 16) as u8])
    })
}

pub fn jpeg(img: &RgbImage, quality: u8) -> Vec<u8> {
    jpeg_with_exif(img, quality, None)
}

/// JPEG with an EXIF APP1 block. `exif` is the raw TIFF payload.
pub fn jpeg_with_exif(img: &RgbImage, quality: u8, exif: Option<&[u8]>) -> Vec<u8> {
    let mut out = Vec::new();
    let mut enc = Encoder::new(&mut out, quality);
    if let Some(exif) = exif {
        enc.add_exif_metadata(exif).unwrap();
    }
    enc.encode(
        img.as_raw(),
        img.width() as u16,
        img.height() as u16,
        ColorType::Rgb,
    )
    .unwrap();
    out
}

/// Minimal big-endian TIFF with the Orientation tag (and optionally a fake GPS string).
pub fn exif_orientation(orientation: u16, extra: &[u8]) -> Vec<u8> {
    let mut t = b"MM\0\x2A\0\0\0\x08".to_vec(); // header, IFD at offset 8
    t.extend_from_slice(&1u16.to_be_bytes()); // 1 entry
    t.extend_from_slice(&0x0112u16.to_be_bytes()); // Orientation
    t.extend_from_slice(&3u16.to_be_bytes()); // SHORT
    t.extend_from_slice(&1u32.to_be_bytes()); // count
    t.extend_from_slice(&orientation.to_be_bytes());
    t.extend_from_slice(&[0, 0]); // padding
    t.extend_from_slice(&0u32.to_be_bytes()); // no next IFD
    t.extend_from_slice(extra);
    t
}

pub fn png_rgb(img: &RgbImage) -> Vec<u8> {
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    out
}

pub fn png_rgba(img: &RgbaImage) -> Vec<u8> {
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgba8,
        )
        .unwrap();
    out
}

pub fn encode_as(img: &DynamicImage, format: image::ImageFormat) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, format).unwrap();
    out.into_inner()
}

/// A PNG whose header claims `w`×`h` (with a valid CRC) but has no pixel data.
pub fn png_header_only(w: u32, h: u32) -> Vec<u8> {
    let mut v = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = b"IHDR".to_vec();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    v.extend_from_slice(&13u32.to_be_bytes());
    v.extend_from_slice(&ihdr);
    v.extend_from_slice(&crc32fast::hash(&ihdr).to_be_bytes());
    // Empty IDAT so the decoder finishes reading the header.
    v.extend_from_slice(&0u32.to_be_bytes());
    v.extend_from_slice(b"IDAT");
    v.extend_from_slice(&crc32fast::hash(b"IDAT").to_be_bytes());
    v
}

pub fn transparent_square(size: u32) -> RgbaImage {
    RgbaImage::from_fn(size, size, |x, _| {
        if x < size / 2 {
            Rgba([0, 0, 0, 0])
        } else {
            Rgba([200, 30, 30, 255])
        }
    })
}

pub fn decode(bytes: &[u8]) -> DynamicImage {
    image::load_from_memory(bytes).expect("output must always be a valid image")
}

pub const KB: u32 = 1024;
