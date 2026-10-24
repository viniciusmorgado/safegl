use anyhow::{ensure, Result};
use safegl::{framebuffer::SimpleFrameBuffer, texture::SrgbTexture2d};

use crate::{Camera, Renderer};

type Pixels = Vec<Vec<(u8, u8, u8, u8)>>;

fn difference(a: &Pixels, b: &Pixels) -> f64 {
    a.iter()
        .flatten()
        .zip(b.iter().flatten())
        .map(|(a, b)| {
            let dr = f64::from(a.0) - f64::from(b.0);
            let dg = f64::from(a.1) - f64::from(b.1);
            let db = f64::from(a.2) - f64::from(b.2);
            dr * dr + dg * dg + db * db
        })
        .sum()
}

// Exercise actual shaders/readbacks: static accumulation, motion, cuts, and resize.
pub fn check(renderer: &mut Renderer) -> Result<()> {
    let color = SrgbTexture2d::empty(&renderer.display, 640, 480)?;
    let mut target = SimpleFrameBuffer::new(&renderer.display, &color)?;
    renderer.deferred.reset_history();
    let mut previous = None;
    let mut raw_variation = 0.0;
    let mut taa_variation = 0.0;
    for frame in 0..48 {
        renderer.draw(&mut target)?;
        let (raw, filtered) = renderer.deferred.validation_pixels();
        if frame == 0 {
            ensure!(
                difference(&raw, &filtered) == 0.0,
                "First frame used stale TAA history"
            );
        }
        if frame >= 24 {
            if let Some((old_raw, old_filtered)) = &previous {
                raw_variation += difference(&raw, old_raw);
                taa_variation += difference(&filtered, old_filtered);
            }
        }
        previous = Some((raw, filtered));
    }
    ensure!(
        raw_variation > 0.0,
        "Jitter did not change rendered samples"
    );
    ensure!(
        taa_variation < raw_variation,
        "TAA did not reduce static temporal variation"
    );
    println!(
        "TAA static variation: {:.1}% of unfiltered jittered rendering",
        taa_variation / raw_variation * 100.0
    );
    for _ in 0..24 {
        renderer.camera.orbit(0.025, 0.004);
        renderer.draw(&mut target)?;
    }
    // A large camera jump must invalidate history automatically.
    renderer.camera.yaw += 1.5;
    renderer.draw(&mut target)?;
    let (raw, filtered) = renderer.deferred.validation_pixels();
    ensure!(
        difference(&raw, &filtered) == 0.0,
        "Camera cut retained history"
    );
    renderer.camera = Camera::default();
    renderer.deferred.reset_history();
    renderer.draw(&mut target)?;
    let (raw, filtered) = renderer.deferred.validation_pixels();
    ensure!(
        difference(&raw, &filtered) == 0.0,
        "Explicit reset retained history"
    );
    let resized = SrgbTexture2d::empty(&renderer.display, 480, 640)?;
    let mut resized_target = SimpleFrameBuffer::new(&renderer.display, &resized)?;
    renderer.draw(&mut resized_target)?;
    let (raw, filtered) = renderer.deferred.validation_pixels();
    ensure!(
        difference(&raw, &filtered) == 0.0,
        "Resize retained incompatible history"
    );
    renderer.display.assert_no_error(None);
    println!("TAA GPU validation passed: accumulation, camera motion, camera cut, reset, resize");
    Ok(())
}
