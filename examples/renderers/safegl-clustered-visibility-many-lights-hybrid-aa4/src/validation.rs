use crate::{visibility::Mode, Camera, Renderer};
use anyhow::{ensure, Context, Result};
use safegl::{framebuffer::SimpleFrameBuffer, texture::SrgbTexture2d};

// Pixels shaded once at their centre differ from the average of four shaded
// samples wherever shading varies inside the pixel (highlights, texture detail):
// that residual is the aliasing the hybrid accepts. Measured with Ferris at most
// 0.032/255 mean and 0.48% of pixels off by more than 1; the bounds leave margin.
const MEAN_TOLERANCE: f64 = 0.1;
const DIFFERING_PERCENT_TOLERANCE: f64 = 1.0;

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

type Image = Vec<Vec<(u8, u8, u8, u8)>>;

/// Renders `mode` and returns the 8-bit sRGB image with the measured per-sample share.
fn render(
    renderer: &mut Renderer,
    (width, height): (u32, u32),
    mode: Mode,
) -> Result<(Image, f64)> {
    renderer.mode = mode;
    let color = SrgbTexture2d::empty(&renderer.display, width, height)?;
    renderer.draw(&mut SimpleFrameBuffer::new(&renderer.display, &color)?)?;
    let (fraction, measured) = renderer
        .visibility
        .wait_supersampled_fraction()
        .or(renderer.visibility.supersampled_fraction())
        .context("No supersampling measurement")?;
    ensure!(measured == mode, "Measurement belongs to another mode");
    Ok((color.read(), fraction))
}

pub fn check_hybrid(renderer: &mut Renderer) -> Result<()> {
    renderer.all_lights = false;
    renderer.highlight = false;
    for (width, height, yaw, pitch, distance) in [
        (1100, 900, 0.15, 0.08, 3.3),
        (641, 479, 1.3, 0.25, 2.0),
        (640, 480, -1.2, -0.3, 1.4),
        (479, 641, 3.2, 0.0, 6.0),
    ] {
        renderer.camera = Camera {
            yaw,
            pitch,
            distance,
        };
        let (reference, full) = render(renderer, (width, height), Mode::Full)?;
        ensure!(full == 1.0, "Full mode must shade every pixel per sample");
        let mut previous = 0.0;
        for mode in [Mode::Edges, Mode::EdgesSpecular] {
            let (image, fraction) = render(renderer, (width, height), mode)?;
            ensure!(
                fraction > 0.0 && fraction < 1.0,
                "{} mode supersampled {:.2}% of pixels; expected a strict subset",
                mode.name(),
                fraction * 100.0
            );
            ensure!(
                fraction >= previous,
                "Specular classification must only add per-sample pixels"
            );
            previous = fraction;
            let mut sum = 0_u64;
            let mut max_error = 0_u8;
            let mut differing = 0_u64;
            for (a, b) in image.iter().flatten().zip(reference.iter().flatten()) {
                let error =
                    a.0.abs_diff(b.0)
                        .max(a.1.abs_diff(b.1))
                        .max(a.2.abs_diff(b.2));
                differing += u64::from(error > 1);
                max_error = max_error.max(error);
                sum += u64::from(a.0.abs_diff(b.0))
                    + u64::from(a.1.abs_diff(b.1))
                    + u64::from(a.2.abs_diff(b.2));
            }
            let pixels = f64::from(width * height);
            let mean = sum as f64 / (pixels * 3.0);
            let differing = differing as f64 / pixels * 100.0;
            println!("Hybrid validation {width}x{height}, yaw {yaw}, distance {distance}, {}: per-sample {:.2}% of pixels; vs full mean RGB error {mean:.4}/255, max {max_error}, {differing:.3}% pixels off by >1", mode.name(), fraction * 100.0);
            ensure!(
                mean < MEAN_TOLERANCE && differing < DIFFERING_PERCENT_TOLERANCE,
                "{} mode differs too much from full per-sample shading",
                mode.name()
            );
        }
    }
    renderer.mode = Mode::Edges;
    renderer.display.assert_no_error(None);
    println!("Hybrid AA GPU validation passed: edges and edges-specular match full per-sample shading across four camera/size cases");
    Ok(())
}
