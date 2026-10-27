-- Checks the levels of SafeGL at compile time. Each case is a small crate that depends on SafeGL
-- with some levels, for some target, and uses some API: it must compile, or fail with an error
-- that contains a given text.
--
-- Usage, from the root of the repository: lua tests/levels/check_levels.lua [case...]
-- Without arguments, every case runs. The cases of a target that rustup hasn't installed are
-- skipped and listed. The crates are written to `target/levels`.

local root = io.popen("pwd"):read("l")
assert(io.open(root .. "/build/levels.rs"), "run this script from the root of the repository")

local WASM = "wasm32-unknown-unknown"
local MACOS = "aarch64-apple-darwin"

-- `code` is the body of the crate; `fail` is a text of the expected error, or nil to compile
local cases = {
  -- every level compiles the API of its version
  { "gl40_has_subroutines", { "gl40" }, nil, [[
    pub fn f(p: &safegl::Program) { let _ = p.get_subroutine_uniforms(); }
  ]] },
  { "gl41_has_program_binaries", { "gl41" }, nil, [[
    pub fn f(p: &safegl::Program) { let _ = p.get_binary(); }
  ]] },
  { "gl42_has_image_units", { "gl42" }, nil, [[
    pub use safegl::uniforms::ImageUnitFormat;
  ]] },
  { "gl42_has_base_instances", { "gl42" }, nil, [[
    pub fn f() -> safegl::DrawParameters<'static> {
        safegl::DrawParameters { base_instance: 1, ..Default::default() }
    }
  ]] },
  { "gl43_has_buffer_clears", { "gl43" }, nil, [[
    pub fn f(b: &safegl::buffer::Buffer<[u32]>) { let _ = b.clear(0); }
  ]] },
  { "gl43_has_compute_shaders", { "gl43" }, nil, [[
    pub use safegl::program::ComputeShader;
  ]] },
  { "gl44_has_texture_clears", { "gl44" }, nil, [[
    pub use safegl::texture::ClearTextureError;
  ]] },
  { "gl45_has_texture_barriers", { "gl45" }, nil, [[
    pub use safegl::texture::texture_barrier;
  ]] },
  { "gl46_has_draw_counts", { "gl46" }, nil, [[
    pub use safegl::index::DrawCount;
  ]] },
  { "gl46_has_spirv", { "gl46" }, nil, [[
    pub use safegl::program::SpirvProgram;
  ]] },
  { "gles30_has_framebuffer_invalidation", { "gles30" }, nil, [[
    pub use safegl::framebuffer::InvalidateAttachments;
  ]] },
  { "gles31_has_compute_shaders", { "gles31" }, nil, [[
    pub use safegl::program::ComputeShader;
  ]] },
  { "gles32_has_geometry_shaders", { "gles32" }, nil, [[
    pub const P: safegl::index::PrimitiveType = safegl::index::PrimitiveType::TrianglesListAdjacency;
  ]] },
  { "gles32_has_copy_image", { "gles32" }, nil, [[
    pub use safegl::texture::copy_image;
  ]] },
  { "webgl_has_framebuffer_invalidation", { "webgl" }, nil, [[
    pub use safegl::framebuffer::InvalidateAttachments;
  ]], WASM },
  { "both_chains_have_both_apis", { "gl46", "gles32" }, nil, [[
    pub use safegl::draw_parameters::PolygonMode;
    pub use safegl::texture::Texture1d;
    pub const P: safegl::index::PrimitiveType = safegl::index::PrimitiveType::Patches { vertices_per_patch: 3 };
  ]] },
  { "no_level_is_gl40", {}, nil, [[
    pub use safegl::draw_parameters::PolygonMode;
  ]] },

  -- a level doesn't compile the API of the versions above it
  { "no_level_has_no_program_binaries", {}, "no method named `get_binary`", [[
    pub fn f(p: &safegl::Program) { let _ = p.get_binary(); }
  ]] },
  { "gl41_has_no_image_units", { "gl41" }, "ImageUnitFormat", [[
    pub use safegl::uniforms::ImageUnitFormat;
  ]] },
  { "gl41_has_no_base_instances", { "gl41" }, "no field named `base_instance`", [[
    pub fn f() -> safegl::DrawParameters<'static> {
        safegl::DrawParameters { base_instance: 1, ..Default::default() }
    }
  ]] },
  { "gl42_has_no_buffer_clears", { "gl42" }, "no method named `clear`", [[
    pub fn f(b: &safegl::buffer::Buffer<[u32]>) { let _ = b.clear(0); }
  ]] },
  { "gl41_has_no_compute_shaders", { "gl41" }, "ComputeShader", [[
    pub use safegl::program::ComputeShader;
  ]] },
  { "gl43_has_no_query_buffers", { "gl43" }, "ToBufferError", [[
    pub use safegl::draw_parameters::ToBufferError;
  ]] },
  { "gl44_has_no_texture_barriers", { "gl44" }, "texture_barrier", [[
    pub use safegl::texture::texture_barrier;
  ]] },
  { "gl45_has_no_draw_counts", { "gl45" }, "DrawCount", [[
    pub use safegl::index::DrawCount;
  ]] },
  { "gl45_has_no_spirv", { "gl45" }, "SpirvProgram", [[
    pub use safegl::program::SpirvProgram;
  ]] },
  { "gles30_has_no_compute_shaders", { "gles30" }, "ComputeShader", [[
    pub use safegl::program::ComputeShader;
  ]] },
  { "gles31_has_no_geometry_shaders", { "gles31" }, "TrianglesListAdjacency", [[
    pub const P: safegl::index::PrimitiveType = safegl::index::PrimitiveType::TrianglesListAdjacency;
  ]] },
  { "gles31_has_no_geometry_shader_sources", { "gles31" }, "expected `NotInLevel`, found `&str`", [[
    pub fn f<F: safegl::backend::Facade>(display: &F) {
        let _ = safegl::Program::from_source(display, "", "", Some(""));
    }
  ]] },
  { "gles31_has_no_copy_image", { "gles31" }, "copy_image", [[
    pub use safegl::texture::copy_image;
  ]] },
  -- desktop OpenGL only, at any OpenGL ES level
  { "gles31_has_no_polygon_mode", { "gles31" }, "PolygonMode", [[
    pub use safegl::draw_parameters::PolygonMode;
  ]] },
  { "gles31_has_no_polygon_mode_field", { "gles31" }, "no field named `polygon_mode`", [[
    pub fn f() -> safegl::DrawParameters<'static> {
        safegl::DrawParameters { polygon_mode: Default::default(), ..Default::default() }
    }
  ]] },
  { "gles32_has_no_1d_textures", { "gles32" }, "Texture1d", [[
    pub use safegl::texture::Texture1d;
  ]] },
  { "gles32_has_no_texture_clears", { "gles32" }, "ClearTextureError", [[
    pub use safegl::texture::ClearTextureError;
  ]] },
  -- OpenGL ES only, at any desktop level
  { "gl46_has_no_primitive_bounding_box", { "gl46" }, "no field named `primitive_bounding_box`", [[
    pub fn f() -> safegl::DrawParameters<'static> {
        safegl::DrawParameters { primitive_bounding_box: (-1.0..1.0, -1.0..1.0, -1.0..1.0, -1.0..1.0), ..Default::default() }
    }
  ]] },
  -- WebGL 2 is the OpenGL ES 3.0 set
  { "webgl_has_no_compute_shaders", { "webgl" }, "ComputeShader", [[
    pub use safegl::program::ComputeShader;
  ]], WASM },
  { "webgl_has_no_multisample_textures", { "webgl" }, "Texture2dMultisample", [[
    pub use safegl::texture::Texture2dMultisample;
  ]], WASM },
  { "webgl_has_no_program_binaries", { "webgl" }, "no method named `get_binary`", [[
    pub fn f(p: &safegl::Program) { let _ = p.get_binary(); }
  ]], WASM },

  -- the combinations of levels
  { "two_desktop_levels", { "gl41", "gl43" },
    "SafeGL levels `gl41` and `gl43` are both selected: select a single desktop OpenGL level", "" },
  { "two_embedded_levels", { "gles30", "gles32" },
    "SafeGL levels `gles30` and `gles32` are both selected: select a single OpenGL ES level", "" },
  { "desktop_level_in_the_browser", { "gl46", "webgl" },
    "the `gl46` level can't be selected in the browser, which only has WebGL 2", "", WASM },
  { "browser_without_webgl", {}, "browsers only have WebGL 2: select the `webgl` level", "", WASM },

  -- macOS stops at OpenGL 4.1, but `gl46` compiles there and fails when creating the context
  { "gl46_compiles_for_macos", { "gl46" }, nil, [[
    pub use safegl::program::SpirvProgram;
  ]], MACOS },
}

local installed = {}
for target in io.popen("rustup target list --installed 2>/dev/null"):lines() do
  installed[target] = true
end

local selected = {}
for i = 1, #arg do selected[arg[i]] = true end

local function write(path, text)
  local f = assert(io.open(path, "w"))
  f:write(text)
  f:close()
end

local function quote(s) return "'" .. s:gsub("'", "'\\''") .. "'" end

local target_dir = root .. "/target/levels"
local lock = io.open(root .. "/Cargo.lock"):read("a")
local failures, skipped, passed = {}, {}, 0

for _, case in ipairs(cases) do
  local name, features, fail, code, target = case[1], case[2], case[3], case[4], case[5]
  if next(selected) == nil or selected[name] then
    if target and not installed[target] then
      skipped[#skipped + 1] = name .. " (target " .. target .. " not installed)"
    else
      local dir = target_dir .. "/cases/" .. name
      os.execute("mkdir -p " .. quote(dir .. "/src"))
      local feature_list = {}
      for _, f in ipairs(features) do feature_list[#feature_list + 1] = '"' .. f .. '"' end
      write(dir .. "/Cargo.toml", ([[
[package]
name = "%s"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
safegl = { path = %q, default-features = false, features = [%s] }

[workspace]
]]):format(name:gsub("_", "-"), root, table.concat(feature_list, ", ")))
      write(dir .. "/Cargo.lock", lock)
      write(dir .. "/src/lib.rs", "#![allow(dead_code, unused_imports)]\n" .. code .. "\n")

      local command = "cd " .. quote(dir) .. " && CARGO_TARGET_DIR=" .. quote(target_dir) ..
        " cargo check --quiet" .. (target and (" --target " .. target) or "") .. " 2>&1"
      local pipe = io.popen(command)
      local output = pipe:read("a")
      local ok = pipe:close()

      local good
      if fail then
        good = not ok and output:find(fail, 1, true) ~= nil
      else
        good = ok == true
      end
      if good then
        passed = passed + 1
        io.write("ok    ", name, "\n")
      else
        failures[#failures + 1] = name
        io.write("FAIL  ", name, ": expected ", fail and ("an error with `" .. fail .. "`") or "success",
          "\n", output, "\n")
      end
    end
  end
end

for _, s in ipairs(skipped) do io.write("skip  ", s, "\n") end
io.write(("%d passed, %d failed, %d skipped\n"):format(passed, #failures, #skipped))
os.exit(#failures == 0 and 0 or 1)
