-- Checks that the OpenGL ES levels of SafeGL compile for the architectures of embedded devices,
-- and runs the tests on the ones this machine can run: natively for i686 (with the 32-bit
-- libraries of EGL and Mesa), and with QEMU for aarch64, given a sysroot of aarch64 with Mesa.
--
-- Usage, from the root of the repository:
--   lua tests/cross/check_targets.lua [check] [test-i686] [test-aarch64 <sysroot>]
-- Without arguments, only the checks run. The tests run on displays without window system
-- (`SAFEGL_TEST_DISPLAY=headless`) of Mesa's software renderer, so that they need neither X11
-- nor Wayland, nor a GPU. The targets must be installed with `rustup target add`; the missing
-- ones are skipped and listed.

local TARGETS = {
  "aarch64-unknown-linux-gnu",
  "armv7-unknown-linux-gnueabihf",
  "riscv64gc-unknown-linux-gnu",
  "aarch64-linux-android",
  "i686-unknown-linux-gnu",
}
local LEVELS = { "gles30", "gles31", "gles32" }

local function run(command)
  print("$ " .. command)
  local ok = os.execute(command)
  return ok == true or ok == 0
end

local function installed_targets()
  local targets = {}
  local pipe = assert(io.popen("rustup target list --installed"))
  for line in pipe:lines() do
    targets[line] = true
  end
  pipe:close()
  return targets
end

local failures, skipped = {}, {}

local function check()
  local installed = installed_targets()
  for _, target in ipairs(TARGETS) do
    if not installed[target] then
      skipped[#skipped + 1] = target
    else
      for _, level in ipairs(LEVELS) do
        local command = ("cargo check --target %s --features %s --all-targets"):format(
          target, level)
        if not run(command) then
          failures[#failures + 1] = target .. " " .. level
        end
      end
    end
  end
end

-- the environment of the tests: displays without window, of Mesa's software renderer
local HEADLESS = "SAFEGL_TEST_DISPLAY=headless SAFEGL_EGL_DEVICE=software "
  .. "LIBGL_ALWAYS_SOFTWARE=1 MESA_SHADER_CACHE_DISABLE=true "
  .. "__EGL_VENDOR_LIBRARY_FILENAMES=/usr/share/glvnd/egl_vendor.d/50_mesa.json"

-- runs `command` and returns whether it succeeded and its output, which it also prints
local function run_captured(command)
  print("$ " .. command)
  local pipe = assert(io.popen(command .. " 2>&1"))
  local output = pipe:read("a")
  local ok = pipe:close()
  io.write(output)
  return ok, output
end

-- The software renderer of Mesa keeps the memory of the code of its shaders after their
-- contexts are destroyed, so that a 32-bit test program that creates many contexts runs out of
-- address space and crashes. The test programs that crash are run again in chunks of tests, each
-- in a new process.
local CHUNK = 10

local function run_in_chunks(env, binary, name)
  local tests = {}
  local list = ("env %s '%s' --list --format terse 2>/dev/null"):format(env, binary)
  local pipe = assert(io.popen(list))
  for line in pipe:lines() do
    tests[#tests + 1] = line:match("^(.-): test$")
  end
  pipe:close()
  local ok = true
  for first = 1, #tests, CHUNK do
    local names = {}
    for i = first, math.min(first + CHUNK - 1, #tests) do
      names[#names + 1] = "'" .. tests[i] .. "'"
    end
    local command = ("env %s '%s' --exact %s"):format(env, binary, table.concat(names, " "))
    ok = run(command) and ok
  end
  if not ok then
    failures[#failures + 1] = name
  end
end

local function test_i686()
  for _, level in ipairs(LEVELS) do
    local command = ("env %s cargo test --no-fail-fast --target i686-unknown-linux-gnu "
      .. "--features %s"):format(HEADLESS, level)
    local ok, output = run_captured(command)
    if not ok then
      -- the programs whose tests failed, and the ones that crashed
      for test in output:gmatch("to rerun pass `%-%-test (%S+)`") do
        local binary = output:match("Running tests/" .. test .. "%.rs %((%S+)%)")
        -- cargo names the program that crashed by its absolute path
        local file = binary and binary:match("[^/]+$")
        local crashed = file
          and output:find("didn't exit successfully: `[^`]*" .. file:gsub("%p", "%%%0")
            .. "[^`]*` %(signal")
        if crashed then
          run_in_chunks(HEADLESS, binary, ("tests i686 %s %s"):format(level, test))
        else
          failures[#failures + 1] = ("tests i686 %s %s"):format(level, test)
        end
      end
    end
  end
end

-- the tests of aarch64 run in QEMU, linked with the cross compiler, with the libraries of the
-- sysroot (a Debian or Ubuntu arm64 root with Mesa's EGL)
local function test_aarch64(sysroot)
  assert(sysroot, "test-aarch64 needs the path of a sysroot of aarch64")
  local env = HEADLESS
    .. " CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc"
    .. (" CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUNNER='qemu-aarch64 -L %s'"):format(sysroot)
    .. (" __EGL_VENDOR_LIBRARY_FILENAMES=%s/usr/share/glvnd/egl_vendor.d/50_mesa.json")
      :format(sysroot)
  for _, level in ipairs(LEVELS) do
    local command = ("env %s cargo test --no-fail-fast --target aarch64-unknown-linux-gnu "
      .. "--features %s"):format(env, level)
    if not run(command) then
      failures[#failures + 1] = "tests aarch64 " .. level
    end
  end
end

local actions = #arg == 0 and { "check" } or arg
local i = 1
while i <= #actions do
  local action = actions[i]
  if action == "check" then
    check()
  elseif action == "test-i686" then
    test_i686()
  elseif action == "test-aarch64" then
    test_aarch64(actions[i + 1])
    i = i + 1
  else
    error("unknown action " .. action)
  end
  i = i + 1
end

for _, target in ipairs(skipped) do
  print("skipped (not installed): " .. target)
end
for _, failure in ipairs(failures) do
  print("FAILED: " .. failure)
end
print(#failures == 0 and "all passed" or (#failures .. " failed"))
os.exit(#failures == 0 and 0 or 1)
