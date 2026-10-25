use anyhow::{ensure, Context, Result};
use glam::Mat4;
use safegl::{
    backend::Facade,
    buffer::{Buffer, BufferMode, BufferType},
    uniform,
};

pub const LIGHT_COUNT: usize = 256;
pub const TILE_SIZE: u32 = 16;
pub const Z_SLICES: u32 = 24;
pub const NEAR: f32 = 0.05;
pub const FAR: f32 = 100.0;
const STRIDE: usize = LIGHT_COUNT + 1;

pub struct Clusters {
    pub lights: Buffer<[[f32; 4]]>,
    pub lists: Buffer<[u32]>,
    compute: safegl::program::ComputeShader,
    pub grid: [u32; 3],
    dimensions: (u32, u32),
}

impl Clusters {
    pub fn new(display: &impl Facade) -> Result<Self> {
        let mut lights = Vec::with_capacity(LIGHT_COUNT * 2);
        let palette = [
            [1.0, 0.38, 0.18],
            [0.22, 0.5, 1.0],
            [0.35, 1.0, 0.55],
            [1.0, 0.75, 0.35],
        ];
        // Exactly the same deterministic 8x8x4 scene as Forward+ many-lights.
        for z in 0..4 {
            for y in 0..8 {
                for x in 0..8 {
                    lights.push([
                        -1.4 + x as f32 * 0.4,
                        -1.25 + y as f32 * (2.5 / 7.0),
                        -0.9 + z as f32 * 0.7,
                        0.55,
                    ]);
                    let color = palette[(x + y * 3 + z) % palette.len()];
                    lights.push([color[0], color[1], color[2], 1.3]);
                }
            }
        }
        Ok(Self {
            lights: Buffer::new(
                display,
                &lights[..],
                BufferType::ShaderStorageBuffer,
                BufferMode::Immutable,
            )?,
            lists: Buffer::empty_array(
                display,
                BufferType::ShaderStorageBuffer,
                STRIDE,
                BufferMode::Dynamic,
            )?,
            compute: safegl::program::ComputeShader::from_source(
                display,
                include_str!("../shaders/clusters.comp"),
            )
            .context("Compiling clustered light culling")?,
            grid: [1, 1, Z_SLICES],
            dimensions: (0, 0),
        })
    }

    pub fn update(
        &mut self,
        display: &impl Facade,
        dimensions: (u32, u32),
        projection: Mat4,
        view: Mat4,
        all_lights: bool,
    ) -> Result<()> {
        if self.dimensions != dimensions {
            self.grid = [
                dimensions.0.div_ceil(TILE_SIZE),
                dimensions.1.div_ceil(TILE_SIZE),
                Z_SLICES,
            ];
            let count = self.grid.iter().map(|&v| v as usize).product::<usize>();
            self.lists = Buffer::empty_array(
                display,
                BufferType::ShaderStorageBuffer,
                count * STRIDE,
                BufferMode::Dynamic,
            )?;
            self.dimensions = dimensions;
            println!("Cluster grid: {}x{}x{}; 16x16 screen tiles, 24 logarithmic depth slices; {:.1} MiB lists",
                self.grid[0], self.grid[1], self.grid[2], (count * STRIDE * 4) as f64 / 1048576.0);
        }
        if !all_lights {
            self.compute.execute(
                uniform! {
                    LightData: &self.lights, ClusterLists: &self.lists,
                    inverse_projection: projection.inverse().to_cols_array_2d(),
                    view: view.to_cols_array_2d(), viewport: [dimensions.0, dimensions.1],
                    grid: self.grid, near_far: [NEAR, FAR],
                },
                self.grid[0],
                self.grid[1],
                self.grid[2],
            );
        }
        Ok(())
    }

    pub fn statistics(&self) -> Result<(f64, u32)> {
        let data = self.lists.read()?;
        let mut nonempty = 0_u64;
        let mut sum = 0_u64;
        let mut maximum = 0;
        for cluster in data.as_chunks::<STRIDE>().0 {
            let count = cluster[0];
            ensure!(count <= LIGHT_COUNT as u32, "Cluster list overflow");
            for &index in &cluster[1..1 + count as usize] {
                ensure!(index < LIGHT_COUNT as u32, "Invalid light index");
            }
            if count > 0 {
                nonempty += 1;
                sum += u64::from(count);
                maximum = maximum.max(count);
            }
        }
        Ok((sum as f64 / nonempty.max(1) as f64, maximum))
    }
}
