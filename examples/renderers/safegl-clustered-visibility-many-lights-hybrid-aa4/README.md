# Clustered visibility — many lights — hybrid AA 4x

Clustered visibility, 256 point lights + directional fill, and a hybrid of MSAA 4×
coverage with selective 4× supersampling. Uses the published `safegl = "0.1.0"`
crate with the `gl46` feature.

## Run

From this directory:

```sh
cargo run --release --locked
```

From the collection directory:

```sh
cargo run --release --locked -p safegl-clustered-visibility-many-lights-hybrid-aa4
```

## Why a hybrid

MSAA and SSAA fix different kinds of aliasing.

- **MSAA** stores four coverage/depth samples per pixel. Shading once per pixel and
  copying the result to the covered samples smooths triangle edges at little cost.
  It does nothing for aliasing inside a triangle: highlights, texture detail, and
  alpha-tested cutouts.
- **SSAA** shades every sample. It fixes all of these, but costs four times the
  shading. The [`-msaa4`](../safegl-clustered-visibility-many-lights-msaa4/) variant
  shades every sample of its MSAA buffer, and the
  [`-ssaa4`](../safegl-clustered-visibility-many-lights-ssaa4/) variant renders at
  twice the width and height. Both pay that cost on every pixel.

This variant keeps MSAA coverage everywhere and pays for per-sample shading only on
the pixels where MSAA alone is not enough. This is how sparse-grid and transparency
supersampling worked. The factor on those pixels is always the sample count (4×),
never a fraction: supersampling here means running the shader once per MSAA sample
instead of once per pixel.

## Modes

Press `M` to cycle modes at runtime, or pass `--mode`. The window title shows the
current mode. `C` (or `--highlight`) toggles an overlay that tints every pixel
shaded per sample, so you can see where the supersampling cost goes.

| Mode | Shaded per sample | Shaded once |
| --- | --- | --- |
| `edges` (default) | Silhouettes, occlusion boundaries, creases, UV seams, mesh boundaries, and alpha-cutout/screen-door materials | Pixels whose samples all lie on one continuous surface |
| `edges-specular` | Everything in `edges`, plus surface pixels where the normal turns fast enough on screen that the highlight would shimmer | The remaining surface pixels |
| `full` | Every pixel. This matches `-msaa4` and serves as the quality reference. | Nothing |

What changes between `edges` and `edges-specular`:

- `edges` only fixes aliasing caused by geometry. Where the surface curves sharply
  relative to the pixel size (rims near silhouettes, the fingers of the claws, distant
  views), the shininess-32 highlight can still be undersampled.
- `edges-specular` also detects those regions. It does so analytically, without
  shading anything. Compared with `full`, it cuts the share of visibly different
  pixels by 2–4× and usually lowers the worst single-pixel difference (see
  Validation). It costs one more classification pass per mesh and shades up to about
  3× more pixels per sample than `edges`.

Neither mode targets texture detail inside a triangle. Mipmaps handle texture
minification. High-contrast texels seen at grazing angles, such as Ferris's eye,
remain the main difference from `full`.

## Pipeline

- **Visibility.** A four-sample R32UI visibility buffer stores the triangle ID.
  - Bit 31 records facing, and bit 30 marks alpha-coverage materials.
  - Rasterization uses four-sample depth24/stencil8.
  - Alpha-tested and screen-door materials use a visibility variant with `sample`
    interpolation. Each sample then runs its own alpha test (transparency
    supersampling).
  - There is no normal, albedo, position, or stored-barycentric G-buffer.
- **Classification.** A fullscreen pass per mesh runs once per pixel, reads the four
  sample IDs, and writes stencil 1 where per-sample shading is needed. A pixel is
  kept on the cheap path when all four samples hold the same triangle, or hold
  same-facing triangles of one mesh that share an indexed vertex with sample 0's
  triangle. Shared vertices share position, normal, and UV, so shading is continuous
  across them.
  - Without the shared-vertex rule, every internal seam of Ferris's 155k-triangle
    mesh would count as an edge. In the default view that would supersample 6.7% of
    the window instead of 1.2%.
  - Creases and UV seams split vertices, so they stay on the per-sample path.
  - Background next to geometry, other meshes, and facing changes do too.
- **Specular classification** (`edges-specular` only). It reconstructs the triangle
  as shading does and computes analytic screen-space derivatives of the unit normal.
  It marks pixels where the normal turns more than ¼ of the Blinn–Phong lobe's
  half-width (acos(0.5^(1/32)) ≈ 0.21 rad) per pixel.
- **Shading.** Two fullscreen draws per mesh are compiled from one `shade.glsl`.
  - **Stencil 0:** the program does not reference `gl_SampleID`, so it runs once per
    pixel. It evaluates sample 0's triangle at the pixel centre, and the hardware
    writes the result to all four samples.
  - **Stencil 1:** the program uses `gl_SampleID` and `gl_SamplePosition`, so it runs
    once per sample. Each sample keeps its own triangle, attributes, and cluster.
  - Both reconstruct perspective-correct attributes from triangle IDs and select
    texture mipmaps with analytical UV derivatives.
- **Resolve.** The four RGBA16F samples are averaged in linear space, then encoded to
  sRGB once.
- **Light culling.** As in the other clustered variants, a compute shader assigns the
  256 light spheres to **16×16 pixel tiles × 24 logarithmic view-depth slices**
  spanning 0.05–100.0. Each cluster reserves 256 indices plus a count, and all lights
  fit. At 1100×900 the grid reserves about 92.5 MiB of index lists.

A `SamplesPassedQuery` on the classification draws counts the per-sample pixels
without stalling the GPU. The window prints the share once per second together with
FPS.

## Measurements

These numbers come from `--benchmark`: 1100×900 offscreen, NVIDIA GeForce RTX 4060,
median of three batches of 180 frames, CPU+GPU throughput. Measure on your own
hardware; the ratios matter more than the absolute values.

| View | Mode | Per-sample pixels | Frame |
| --- | --- | --- | --- |
| Default | `edges` | 1.18% | 1.82 ms |
| Default | `edges-specular` | 2.96% | 1.86 ms |
| Default | `full` | 100% | 1.94 ms |
| Close (distance 1.4) | `edges` | 1.64% | 2.91 ms |
| Close (distance 1.4) | `edges-specular` | 4.51% | 2.92 ms |
| Close (distance 1.4) | `full` | 100% | 3.28 ms |

For comparison, `-ssaa4` measured 1.95 ms in the default view on the same machine.

Timing each pass with a `glFinish` after it (close view) shows where the saving comes
from:

- **Shading** drops from 0.63 ms (`full`) to 0.25 ms (`edges`) and 0.28 ms
  (`edges-specular`). Classification adds about 0.05 and 0.09 ms respectively.
- **Light culling** takes 1.4–2.3 ms in every mode. That cost is shared with the
  other clustered variants, and it dominates the frame for this lightweight
  Blinn–Phong scene. With heavier materials (PBR, shadows, more texture fetches),
  shading becomes a larger share of the frame, and the hybrid's saving grows with it.

## Model and materials

All examples load the shared `../assets/ferris.glb`, resolved relative to the
package directory at compile time. Keep the collection together when moving it.
See [ATTRIBUTION.md](../ATTRIBUTION.md) for the asset information.

The loader applies node transforms, groups static primitives by material, and
centers/scales the model uniformly to a two-unit largest dimension. Double-sided
materials retain both faces. Ferris uses an opaque base-color texture.

Lighting remains classical Blinn–Phong with base-color textures and vertex normals.
Normal maps, metallic/roughness maps, shadows, skinning, and animation are not used.
The loader also supports alpha cutouts and screen-door coverage for other models.
Ferris uses neither, so the alpha-coverage supersampling path is not exercised by
this asset.

## Controls and graphics defaults

OpenGL **4.6 Core** is required and checked at startup. VSync is disabled and FPS
is uncapped. A top-left counter measures presented frames per second.

- Left-drag or arrow keys: orbit.
- Mouse wheel or `+` / `-`: zoom.
- `M`: next anti-aliasing mode.
- `C`: classification overlay.
- `R`: reset the camera.
- `Esc`: quit.

## Validation

```sh
cargo fmt --check
cargo build --release --locked
cargo clippy --release --locked -- -D warnings
cargo run --release --locked -- --frames 120
cargo run --release --locked -- --screenshot preview.png
cargo run --release --locked -- --validate-culling
cargo run --release --locked -- --validate-hybrid
cargo run --release --locked -- --benchmark
```

`--frames` renders a bounded number of window frames and checks OpenGL errors.
`--screenshot` warms up rendering, saves a 1100×900 PNG, and exits. Combine it with
`--mode` and `--highlight` to capture any mode or the overlay. All of these options
require a desktop graphics session. Run `--help` for the complete option list.

- `--validate-culling`: Compare culled lighting with evaluating all 256 lights across
  camera and viewport cases.
- `--validate-hybrid`: Render `edges` and `edges-specular` against `full` in four
  camera/viewport cases. The check asserts that:
  - each hybrid supersamples a strict subset of the pixels;
  - `edges-specular` supersamples at least as many pixels as `edges`;
  - the mean RGB difference stays below 0.1/255;
  - fewer than 1% of pixels differ by more than one 8-bit step.

  Measured with Ferris, the mean difference was at most 0.031/255 (`edges`) and
  0.020/255 (`edges-specular`). Pixels off by more than one step were at most 0.48%
  and 0.21%. The worst single pixels (up to 73 and 53 steps) sit on the eye texture
  and on tight highlights: the in-surface aliasing that only full supersampling
  removes.
