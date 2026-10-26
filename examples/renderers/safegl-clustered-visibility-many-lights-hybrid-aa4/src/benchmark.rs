use crate::{visibility::Mode, Camera, Renderer};
use anyhow::Result;
use safegl::{backend::Facade, framebuffer::SimpleFrameBuffer, texture::SrgbTexture2d};
pub fn run(renderer: &mut Renderer) -> Result<()> {
    renderer.highlight = false;
    let color = SrgbTexture2d::empty(&renderer.display, 1100, 900)?;
    let mut output = SimpleFrameBuffer::new(&renderer.display, &color)?;
    // The default view leaves most of the window as background; the close view
    // fills it with Ferris, so shading is a larger share of the frame.
    for (view, distance) in [
        ("default view", Camera::default().distance),
        ("close view", 1.4),
    ] {
        for mode in Mode::ALL {
            renderer.camera = Camera {
                distance,
                ..Camera::default()
            };
            renderer.mode = mode;
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
            let fraction = renderer
                .visibility
                .wait_supersampled_fraction()
                .or(renderer.visibility.supersampled_fraction())
                .map_or(f64::NAN, |(fraction, _)| fraction * 100.0);
            println!("BENCHMARK {} --mode {}, {view}: 1100x900 output, 60 warmup + 3x180 frames, median {:.3} ms/frame, {:.1} FPS; range {:.3}..{:.3} ms/frame; per-sample shading on {fraction:.2}% of pixels; CPU+GPU throughput, no swap/compositor", env!("CARGO_PKG_NAME"), mode.name(), batches[1],1000.0/batches[1],batches[0],batches[2]);
        }
    }
    renderer.display.assert_no_error(None);
    Ok(())
}
