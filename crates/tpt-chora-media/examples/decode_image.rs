//! Phase 6 milestone: decode an image into GPU-ready texture
//! format and report the result, demonstrating the asset
//! pipeline's image decoding path.
//!
//! Run with: `cargo run -p tpt-chora-media --example decode_image`

use tpt_chora_media::decode::ImageFormat;
use tpt_chora_media::ImageDecoder;

fn main() {
    let decoder = ImageDecoder::new();

    let test_image = generate_test_png(64, 64);

    println!("=== Image Decode ===");
    println!("input bytes: {} (PNG)", test_image.len());
    println!();

    match decoder.decode(&test_image) {
        Ok(decoded) => {
            println!("decoded successfully:");
            println!("  width:  {}", decoded.width);
            println!("  height: {}", decoded.height);
            println!("  format: {:?}", decoded.format);
            println!("  pixels: {} bytes", decoded.data.len());
            println!();

            assert_eq!(decoded.width, 64);
            assert_eq!(decoded.height, 64);
            assert_eq!(decoded.format, ImageFormat::Rgba8);
            assert_eq!(decoded.data.len(), (64 * 64 * 4) as usize);
            println!("all assertions passed");
        }
        Err(e) => {
            println!("decode failed: {}", e);
        }
    }
}

fn generate_test_png(width: u32, height: u32) -> Vec<u8> {
    let img = image::RgbaImage::from_fn(width, height, |x, y| {
        let r = ((x as f32 / width as f32) * 255.0) as u8;
        let g = ((y as f32 / height as f32) * 255.0) as u8;
        image::Rgba([r, g, 0, 255])
    });
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgba8(img)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .expect("encode test PNG");
    bytes
}
