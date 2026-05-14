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
