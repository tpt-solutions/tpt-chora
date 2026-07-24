//! Phase 1 milestone: render a triangle and a GPU-tessellated vector path
//! through the render graph to an off-screen target, then write the
//! result to a PNG so it can be inspected outside of a GPU debugger.
//!
//! Run with: `cargo run -p tpt-chora-render --example triangle_and_path`

use tpt_chora_render::Renderer;

fn main() {
    const WIDTH: u32 = 512;
    const HEIGHT: u32 = 512;

    let renderer = Renderer::new_headless(WIDTH, HEIGHT).expect(
        "failed to create a GPU context (no Vulkan/Metal/DX12/GL adapter available, \
         not even wgpu's software fallback)",
    );

    let pixels = renderer
        .render_frame()
        .expect("failed to render the milestone frame");

    assert_eq!(pixels.len(), (WIDTH * HEIGHT * 4) as usize);

    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/output");
    std::fs::create_dir_all(&out_dir).expect("failed to create examples/output/");
    let out_path = out_dir.join("triangle_and_path.png");

    let file = std::fs::File::create(&out_path).expect("failed to create output PNG file");
    let writer = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, WIDTH, HEIGHT);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("failed to write PNG header");
    writer
        .write_image_data(&pixels)
        .expect("failed to write PNG pixel data");

    println!("wrote {}", out_path.display());
}
