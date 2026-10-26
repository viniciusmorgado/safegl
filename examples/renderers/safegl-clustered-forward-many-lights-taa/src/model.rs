use anyhow::{bail, ensure, Context, Result};
use glam::{Mat3, Mat4, Vec3};
use safegl::{
    glutin::surface::WindowSurface,
    index::PrimitiveType,
    texture::{RawImage2d, SrgbTexture2d},
    Display, IndexBuffer, VertexBuffer,
};
use std::path::Path;

#[derive(Clone, Copy)]
pub struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    tex_coord: [f32; 2],
}
safegl::implement_vertex!(Vertex, position, normal, tex_coord);

pub struct Mesh {
    pub vertices: VertexBuffer<Vertex>,
    pub indices: IndexBuffer<u32>,
    pub texture: SrgbTexture2d,
    pub base_color: [f32; 4],
    pub double_sided: bool,
    pub alpha_mode: i32,
    pub alpha_cutoff: f32,
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub transform: Mat4,
}

impl Model {
    pub fn load(display: &Display<WindowSurface>, path: &Path) -> Result<Self> {
        let (document, buffers, images) =
            gltf::import(path).with_context(|| format!("Importing {}", path.display()))?;
        ensure!(
            document.skins().len() == 0,
            "This static viewer does not support skinned meshes"
        );
        let scene = document
            .default_scene()
            .or_else(|| document.scenes().next())
            .context("Model has no scene")?;
        let mut instances = Vec::new();
        for node in scene.nodes() {
            collect_meshes(node, Mat4::IDENTITY, &mut instances);
        }
        let mut meshes = Vec::new();
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        let mut vertex_count = 0;
        let mut triangle_count = 0;
        // Bake node transforms, then batch static geometry by glTF material.
        // Each material uploads its base-color texture once.
        let mut batches = std::collections::BTreeMap::new();
        for (mesh, world) in instances {
            ensure!(world.determinant().abs() > 1e-8, "Singular mesh transform");
            let normal_transform = Mat3::from_mat4(world).inverse().transpose();
            for primitive in mesh.primitives() {
                ensure!(
                    primitive.mode() == gltf::mesh::Mode::Triangles,
                    "Only triangle primitives are supported"
                );
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()].0));
                let positions: Vec<_> = reader
                    .read_positions()
                    .context("Missing positions")?
                    .collect();
                let normals: Vec<_> = reader
                    .read_normals()
                    .context("Missing vertex normals")?
                    .collect();
                let tex_coords: Vec<_> = reader
                    .read_tex_coords(0)
                    .context("Missing TEXCOORD_0")?
                    .into_f32()
                    .collect();
                ensure!(
                    positions.len() == normals.len() && positions.len() == tex_coords.len(),
                    "Mismatched vertex attributes"
                );
                let vertices: Vec<_> = positions
                    .iter()
                    .zip(&normals)
                    .zip(&tex_coords)
                    .map(|((p, n), uv)| {
                        let position = world.transform_point3(Vec3::from_array(*p));
                        min = min.min(position);
                        max = max.max(position);
                        Vertex {
                            position: position.to_array(),
                            normal: (normal_transform * Vec3::from_array(*n))
                                .normalize_or_zero()
                                .to_array(),
                            tex_coord: *uv,
                        }
                    })
                    .collect();
                let mut indices: Vec<u32> = reader
                    .read_indices()
                    .map(|indices| indices.into_u32().collect())
                    .unwrap_or_else(|| (0..vertices.len() as u32).collect());
                ensure!(
                    indices.len().is_multiple_of(3),
                    "Incomplete triangle indices"
                );
                ensure!(
                    indices.iter().all(|&i| (i as usize) < vertices.len()),
                    "Index outside vertex buffer"
                );
                if world.determinant() < 0.0 {
                    for triangle in indices.as_chunks_mut::<3>().0 {
                        triangle.swap(1, 2);
                    }
                }
                let material = primitive.material();
                let batch = batches
                    .entry(material.index())
                    .or_insert_with(|| (Vec::new(), Vec::new(), material.clone()));
                let offset = u32::try_from(batch.0.len())?;
                for index in indices {
                    batch
                        .1
                        .push(index.checked_add(offset).context("Batch index overflow")?);
                }
                batch.0.extend(vertices);
            }
        }
        for (_, (vertices, indices, material)) in batches {
            let pbr = material.pbr_metallic_roughness();
            let texture = if let Some(info) = pbr.base_color_texture() {
                ensure!(info.tex_coord() == 0, "Only TEXCOORD_0 is supported");
                let image = &images[info.texture().source().index()];
                let pixels = rgba8(image)?;
                // glTF UVs address top-to-bottom image rows: upload without flipping.
                SrgbTexture2d::new(
                    display,
                    RawImage2d::from_raw_rgba(pixels, (image.width, image.height)),
                )?
            } else {
                SrgbTexture2d::new(display, RawImage2d::from_raw_rgba(vec![255; 4], (1, 1)))?
            };
            vertex_count += vertices.len();
            triangle_count += indices.len() / 3;
            meshes.push(Mesh {
                vertices: VertexBuffer::new(display, &vertices)?,
                indices: IndexBuffer::new(display, PrimitiveType::TrianglesList, &indices)?,
                texture,
                base_color: pbr.base_color_factor(),
                double_sided: material.double_sided(),
                alpha_mode: match material.alpha_mode() {
                    gltf::material::AlphaMode::Opaque => 0,
                    gltf::material::AlphaMode::Mask => 1,
                    gltf::material::AlphaMode::Blend => 2,
                },
                alpha_cutoff: material.alpha_cutoff().unwrap_or(0.5),
            });
        }
        ensure!(!meshes.is_empty(), "Scene has no drawable meshes");
        let extent = (max - min).max_element();
        ensure!(extent.is_finite() && extent > 0.0, "Invalid model bounds");
        let center = (min + max) * 0.5;
        let transform =
            Mat4::from_scale(Vec3::splat(2.0 / extent)) * Mat4::from_translation(-center);
        println!(
            "Loaded {}: {} draw(s), {} vertices, {} triangles",
            path.display(),
            meshes.len(),
            vertex_count,
            triangle_count
        );
        println!("Bounds: {min:?} .. {max:?}; centered and scaled to a 2-unit largest dimension");
        Ok(Self { meshes, transform })
    }
}

fn collect_meshes<'a>(node: gltf::Node<'a>, parent: Mat4, out: &mut Vec<(gltf::Mesh<'a>, Mat4)>) {
    let world = parent * Mat4::from_cols_array_2d(&node.transform().matrix());
    if let Some(mesh) = node.mesh() {
        out.push((mesh, world));
    }
    for child in node.children() {
        collect_meshes(child, world, out);
    }
}

fn rgba8(image: &gltf::image::Data) -> Result<Vec<u8>> {
    use gltf::image::Format;
    match image.format {
        Format::R8G8B8A8 => Ok(image.pixels.clone()),
        Format::R8G8B8 => Ok(image
            .pixels
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect()),
        other => bail!("Unsupported base-color image format: {other:?}; expected RGB8 or RGBA8"),
    }
}
