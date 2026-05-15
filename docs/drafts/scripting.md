# Scripting (Draft)

This draft summarizes current scripting direction for `.marmot` templates.

## Package Layout

Scripts live in:

```text
scripts/<ULID>.lua
```

inside the `.marmot` package.

- Script filename matches frame identity from PSL `frames` block (`<u32> <ULID>`).

## Core Data Access

Design intent is to expose slot data to Lua via:

```lua
local upc = data.getSlot("upc")
```

- `data.getSlot("slot_name")` reads external template input slot values.
- Missing/invalid values should produce clear runtime errors or explicit `nil` handling rules.

## Frame Mutations

### `frame.visible`

- Boolean flag controlling whether frame should render.
- Setting `frame.visible = false` means renderer skips frame draw commands.
- Type must remain strict boolean at runtime.

Example:

```lua
if data.getSlot("upc") ~= "" then
    frame.visible = false
end
```

### `frame.value`

- Mutable value for value-bearing frame types (for example text/image/barcode payloads).
- Used for script-time transforms before render.
- Type validation should enforce property compatibility.

Example:

```lua
frame.value = trim(" asdf")
```

## Runtime Flow (Current Direction)

1. Load template + scripts + assets + fonts.
2. Tokenize
3. Parse
    1. Bind input slots.
    2. Bind frames.
4. Render
    1. Parse job data (JSON).
    2. Run Lua scripts on frames.
    3. Validate mutated frame properties (`visible`, `value`, types).
    4. Render using final frame state.

## Error Policy

- Script errors fail render hard.
- Invalid property assignments fail render hard.
- No script for frame is valid; default frame state remains `visible = true`.

## Implementation Notes

This section captures concrete implementation details for the first Lua integration pass.

### Runtime Contract (v1)

- `frame.value` is a string override.
- The override is consumed by value-bearing draw ops in that frame:
  - `textbox`
  - `image` (asset alias)
  - `barcode` (encoded payload)
- `frame.value = nil` clears the override.
- `frame.visible` remains strict boolean (`true` / `false`).

### Lua Engine Choice

- Use `mlua` with vendored Lua 5.4 for deterministic local/dev builds.

```toml
mlua = { version = "0.10", features = ["lua54", "vendored"] }
```

### Package + Script Loading

- Script files remain `scripts/<ULID>.lua` inside `.marmot` package.
- Build a script map during context setup:
  - Key: frame id (`FrameDecl.id`, ULID string)
  - Value: full script source
- Missing script for a frame is a no-op.
- Unknown script file (no matching frame id) should fail package/context validation.

### Render Flow Integration

At render time:

1. Build initial frame runtime state for all frames.
2. For each frame declaration, if script exists for `frame.id`:
   1. Create Lua state (or reuse worker-local Lua state).
   2. Bind globals: `data`, `frame`.
   3. Execute script.
   4. Validate resulting frame runtime state.
3. Execute draw pass using final frame runtime state.

### `data` API

- Expose `data` as a Lua table with function:

```lua
local v = data.getSlot("slot_name")
```

- `getSlot` behavior:
  - Reads top-level slot value from JSON object.
  - Returns Lua `string`, `number`, `boolean`, or `nil`.
  - Arrays/objects are invalid for v1 and should fail with clear runtime error.

### `frame` API

- Expose `frame` as Lua userdata with strict fields:
  - `frame.visible: boolean`
  - `frame.value: string | nil`
- Invalid assignments fail hard:
  - `frame.visible = "no"` -> error
  - `frame.value = 123` -> error

### Draw Op Override Rules

When `frame.value` is set to non-empty string:

- `DrawOp::TextBox`: use `frame.value` text instead of evaluating draw op text expression.
- `DrawOp::Image`: use `frame.value` as asset alias instead of evaluated asset text expression.
- `DrawOp::Barcode`: use `frame.value` as barcode payload instead of evaluated value expression.

When `frame.value` is `nil` or empty string, fall back to existing draw-op evaluation behavior.

### Renderer State Shape

Extend frame runtime state to support script override explicitly:

- `visible: bool`
- `value_override: Option<String>`

Keep warning behavior for empty values, but warn only when override came from script and is empty.

### Validation + Error Messages

- Script compile/runtime errors include frame id and frame index.
- Type assignment errors include property name and expected type.
- Invalid override usage errors include draw op kind and frame id/index.
- Continue to fail render immediately on first scripting error.

### Threading / Batch Notes

- Batch workers already render independently; keep Lua state worker-local.
- Do not share a single Lua state across threads.
- Keep script sources immutable (`Arc<HashMap<String, String>>`) in render context.

### Test Plan (minimum)

- Frame script toggles visibility off and skips draw.
- Frame script sets text override and affects `textbox`.
- Frame script sets image alias override and affects `image`.
- Frame script sets barcode payload override and affects `barcode`.
- Invalid `frame.visible` assignment fails render.
- Invalid `frame.value` assignment fails render.
- Script runtime error fails render.
- Missing script for frame is no-op.
