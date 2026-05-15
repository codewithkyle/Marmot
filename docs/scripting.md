# Marmot Scripting

This document describes scripting behavior implemented today.

## Package Layout

Scripts are packaged under:

```text
scripts/<frame_id>.lua
```

- `<frame_id>` is the frame id from PSL `frames begin ... end`.
- Missing script for a frame is valid (no-op).
- Unknown script file (no matching frame id) fails context build.
- Non-`.lua` files in `scripts/` fail context build.

## Runtime Model

Render flow (scripting parts):

1. Build frame runtime state (`visible=true`, `value_override=nil`) for all frames.
2. For each planned scripted frame, execute Lua script with globals `data` and `frame`.
3. Apply final runtime frame state to draw pass.

Scripts run in an isolated Lua environment with a restricted global base.

## `data` API

```lua
local v = data.getSlot("slot_name")
```

`data.getSlot` behavior:

- Reads top-level value from JSON object.
- Returns Lua `string`, `number`, `boolean`, or `nil`.
- Missing key returns `nil`.
- JSON arrays/objects fail hard with runtime error.

## `frame` API

`frame` is a strict userdata with mutable fields:

- `frame.visible: boolean`
- `frame.value: string | nil`

Rules:

- Invalid `frame.visible` assignment fails render hard.
- Invalid `frame.value` assignment fails render hard.
- `frame.value = nil` clears override.

## Override Semantics

When `frame.value` is non-empty string:

- `textbox` uses script override text.
- `image` uses script override as asset alias.
- `barcode` uses script override as barcode payload.

When `frame.value` is `nil` or empty string, renderer falls back to normal PSL evaluation.

## Error Policy

- Script compile/runtime/type errors fail render immediately.
- Errors include frame id and frame index context.
