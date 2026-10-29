-- Runs the tests of SafeGL on an embedded device, at each OpenGL ES level that its driver
-- creates, and writes a report of the driver and of the results, to share in an issue.
--
-- Usage, on the device, from the root of the repository:
--   lua tests/cross/embedded_device_tests.lua [--window] [--levels gles30,gles31,gles32]
--                                             [--out <directory>]
-- By default, the tests draw on displays without window system (`SAFEGL_TEST_DISPLAY=headless`),
-- on the first EGL device of the driver; `--window` uses windows of X11 or Wayland instead. The
-- report goes to `target/device-report/report.md`, with the output of each run next to it.

local window, levels, out = false, { "gles30", "gles31", "gles32" }, "target/device-report"
local i = 1
while i <= #arg do
  if arg[i] == "--window" then
    window = true
  elseif arg[i] == "--levels" then
    levels = {}
    for level in arg[i + 1]:gmatch("[^,]+") do
      levels[#levels + 1] = level
    end
    i = i + 1
  elseif arg[i] == "--out" then
    out = arg[i + 1]
    i = i + 1
  else
    error("unknown argument " .. arg[i])
  end
  i = i + 1
end
assert(io.open("build/levels.rs"), "run this script from the root of the repository")
os.execute("mkdir -p '" .. out .. "'")

local env = window and "" or "SAFEGL_TEST_DISPLAY=headless "

-- runs `command`, writes its output to `file`, and returns whether it succeeded and the output
local function run(command, file)
  print("$ " .. command)
  local ok = os.execute(command .. " > '" .. out .. "/" .. file .. "' 2>&1")
  local f = assert(io.open(out .. "/" .. file))
  local output = f:read("a")
  f:close()
  return ok == true or ok == 0, output
end

local report = { "# SafeGL on a device", "" }
local function add(line)
  report[#report + 1] = line
end
local machine = io.popen("uname -srm"):read("l")
add("Machine: `" .. machine .. "`, displays: " .. (window and "windows" or "without window"))
add("")

for _, level in ipairs(levels) do
  add("## " .. level)
  add("")
  -- the driver creates a context of the level, or the level is skipped
  local ok, output = run(("env %scargo test --features %s --test device_report "
    .. "-- --ignored --nocapture"):format(env, level), level .. "-driver.txt")
  if not ok then
    add("No context of this level: see `" .. level .. "-driver.txt`.")
    add("")
  else
    for _, name in ipairs({ "version", "vendor", "renderer" }) do
      local value = output:match("\n" .. name .. ": ([^\n]*)")
      add(("- %s: `%s`"):format(name, value or "?"))
    end
    for line in output:gmatch("precision of ([^\n]*)") do
      add("- precision of " .. line)
    end
    add("")
    local _, tests = run(("env %scargo test --no-fail-fast --features %s"):format(env, level),
      level .. "-tests.txt")
    local passed, failed = 0, 0
    for p, f in tests:gmatch("test result: %a+%. (%d+) passed; (%d+) failed") do
      passed, failed = passed + tonumber(p), failed + tonumber(f)
    end
    add(("%d passed, %d failed (output: `%s-tests.txt`)"):format(passed, failed, level))
    add("")
    -- the names of the failed tests are listed under `failures:`, indented, before a blank
    -- line; the details of the failures come under another `failures:`, not indented
    for list in tests:gmatch("\nfailures:\n(.-)\n\n") do
      for name in list:gmatch("    ([%w_:]+)") do
        add("- failed: `" .. name .. "`")
      end
    end
    add("")
  end
end

local f = assert(io.open(out .. "/report.md", "w"))
f:write(table.concat(report, "\n"), "\n")
f:close()
print("report: " .. out .. "/report.md")
