-- Compiles the GLSL sources of this directory to the SPIR-V modules that the tests load, for
-- OpenGL (`glslangValidator -G`), and writes a copy of each module without its debug names
-- (`spirv-opt --strip-debug`), named `<name>.stripped.spv`. Needs glslang and SPIRV-Tools.
--   lua tests/fixture/spirv/build.lua
local dir = arg[0]:match("^(.*)/") or "."
local sources = { "textured.vert", "textured.frag", "capture.vert", "capture.frag",
  "work_group.comp" }

local function run(command)
  local ok = os.execute(command)
  if not ok then
    error("failed: " .. command)
  end
end

for _, source in ipairs(sources) do
  local input = dir .. "/" .. source
  local output = input .. ".spv"
  local stripped = input .. ".stripped.spv"
  run(("glslangValidator -G -o %q %q > /dev/null"):format(output, input))
  run(("spirv-opt --strip-debug -o %q %q"):format(stripped, output))
  run(("spirv-val --target-env opengl4.5 %q"):format(stripped))
  print(output)
end
