use crate::{Camera, Renderer};
use anyhow::{ensure, Result};
use safegl::{framebuffer::SimpleFrameBuffer, texture::SrgbTexture2d};

pub fn check(renderer: &mut Renderer) -> Result<()> {
    // Odd sizes exercise partial edge tiles; side/close views exercise depth bounds.
    for (width, height, yaw, pitch, distance) in [
        (641, 479, 0.15, 0.08, 3.3),
        (479, 641, 1.3, 0.25, 2.0),
        (640, 480, -1.2, -0.3, 1.4),
        (640, 480, 3.2, 0.0, 3.3),
        (640, 480, 0.15, 0.08, 12.0),
    ] {
        renderer.camera = Camera {
            yaw,
            pitch,
            distance,
        };
        let tiled = SrgbTexture2d::empty(&renderer.display, width, height)?;
        let reference = SrgbTexture2d::empty(&renderer.display, width, height)?;
        renderer.all_lights = false;
        renderer.draw(&mut SimpleFrameBuffer::new(&renderer.display, &tiled)?)?;
        let (average, maximum) = renderer.visibility.cluster_statistics()?;
        ensure!(
            average > 0.0 && average < 256.0,
            "Light culling produced no useful lists"
        );
        renderer.all_lights = true;
        renderer.draw(&mut SimpleFrameBuffer::new(&renderer.display, &reference)?)?;
        let a: Vec<Vec<(u8, u8, u8, u8)>> = tiled.read();
        let b: Vec<Vec<(u8, u8, u8, u8)>> = reference.read();
        let background = b[0][0];
        let visible_pixels = b
            .iter()
            .take(height.saturating_sub(48) as usize)
            .flatten()
            .filter(|p| {
                p.0.abs_diff(background.0) > 4
                    || p.1.abs_diff(background.1) > 4
                    || p.2.abs_diff(background.2) > 4
            })
            .count();
        ensure!(
            visible_pixels > 100,
            "Reference render is blank; comparison is invalid"
        );
        let mut sum = 0_u64;
        let mut max_error = 0_u8;
        for (a, b) in a.iter().flatten().zip(b.iter().flatten()) {
            for d in [a.0.abs_diff(b.0), a.1.abs_diff(b.1), a.2.abs_diff(b.2)] {
                sum += u64::from(d);
                max_error = max_error.max(d);
            }
        }
        let mean = sum as f64 / (width * height * 3) as f64;
        println!("Culling validation {width}x{height}, yaw {yaw}: mean RGB error {mean:.5}/255, max {max_error}; avg {average:.1}, max {maximum} lights/nonempty cluster");
        ensure!(
            mean < 0.1 && max_error <= 3,
            "Culled shading differs from all-lights reference"
        );
    }
    renderer.display.assert_no_error(None);
    println!("Clustered visibility GPU validation passed: culling matches all-lights shading across five camera/size cases");
    Ok(())
}
