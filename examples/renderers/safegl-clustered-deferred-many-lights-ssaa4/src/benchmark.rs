use crate::{Camera, Renderer};
use anyhow::Result;
use safegl::{backend::Facade, framebuffer::SimpleFrameBuffer, texture::SrgbTexture2d};
pub fn run(renderer: &mut Renderer) -> Result<()> {
    renderer.camera = Camera::default();
    let color = SrgbTexture2d::empty(&renderer.display, 1100, 900)?;
    let mut output = SimpleFrameBuffer::new(&renderer.display, &color)?;
    for _ in 0..60 {
        renderer.draw(&mut output)?;
    }
    renderer.display.get_context().finish();
    let mut batches = [0.0_f64; 3];
    for milliseconds in &mut batches {
        let started = std::time::Instant::now();
        for _ in 0..180 {
            renderer.draw(&mut output)?;
        }
        renderer.display.get_context().finish();
        *milliseconds = started.elapsed().as_secs_f64() * 1000.0 / 180.0;
    }
    batches.sort_by(f64::total_cmp);
    renderer.display.assert_no_error(None);
    println!("BENCHMARK {}: 1100x900 output, 60 warmup + 3x180 frames, median {:.3} ms/frame, {:.1} FPS; range {:.3}..{:.3} ms/frame; CPU+GPU throughput, no swap/compositor", env!("CARGO_PKG_NAME"), batches[1],1000.0/batches[1],batches[0],batches[2]);
    Ok(())
}
