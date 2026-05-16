# Marmot Scripting

This document describes scripting behavior implemented today.

## Package Layout

Scripts are packaged under:

```text
scripts/<script_id>.lua
```

- `<script_id>` can be either a layer id from PSL `layers begin ... end` or a frame id from PSL `frames begin ... end`.
- Missing script for a layer/frame is valid (no-op).
- Unknown script file (no matching layer/frame id) fails context build.
- Non-`.lua` files in `scripts/` fail context build.

## Runtime Model

Render flow (scripting parts):

1. Build layer runtime state (`visible=true`) for all layers.
2. Build frame runtime state (`visible=true`, `value_override=nil`) for all frames.
3. For each planned scripted layer, execute Lua script with globals `data` and `layer`.
4. For each planned scripted frame, execute Lua script with globals `data` and `frame`.
5. Apply final layer/frame runtime state to draw pass.

Scripts run in an isolated Lua environment with a restricted global base.

## Built-in Helpers

The runtime exposes helper functions directly in script scope.

Formatting and math helpers:

- `percent(part, total) -> string`
  - Computes `(part / total) * 100` and appends `%`.
  - Example: `percent(5.0, 10.0)` -> `"50%"`.
  - `total` must not be `0`.
- `currency(amount) -> string`
  - Formats dollars with 2 decimals.
  - Example: `currency(12.5)` -> `"$12.50"`.
- `round(value, places) -> number`
  - Rounds to decimal places.
  - Example: `round(12.345, 2)` -> `12.35`.
- `save_amount(regular, sale) -> number`
  - Returns `regular - sale`, rounded to 2 decimals.
  - Example: `save_amount(9.99, 7.49)` -> `2.5`.
- `unit_price(total, qty, unit) -> string`
  - Example: `unit_price(5.99, 16, "oz")` -> `"$0.37/oz"`.
  - `qty` must be greater than `0`.
- `unit_price_each(total, count) -> string`
  - Example: `unit_price_each(10.00, 4)` -> `"$2.50 ea"`.
  - `count` must be greater than `0`.

Value and string helpers:

- `default(value, fallback) -> any`
  - Returns `fallback` only when `value` is `nil`.
  - Example: `default(nil, "N/A")` -> `"N/A"`.
- `concat(...) -> string`
  - Concatenates arguments after string conversion.
  - Example: `concat("BUY ", 1, " GET ", 1)` -> `"BUY 1 GET 1"`.
- `pad_left(value, width, pad) -> string`
  - Example: `pad_left(42, 5, "0")` -> `"00042"`.
  - `pad` must not be empty.
- `pad_right(value, width, pad) -> string`
  - Example: `pad_right(42, 5, "0")` -> `"42000"`.
  - `pad` must not be empty.
- `truncate(value, max_len) -> string`
  - Truncates to `max_len` characters and appends `…` when needed.
  - Example: `truncate("Organic Honeycrisp Apples", 12)` -> `"Organic Hon…"`.
- `trim(value) -> string`
  - Trims both sides.
- `trim_left(value) -> string`
  - Trims leading whitespace.
- `trim_right(value) -> string`
  - Trims trailing whitespace.

Structured/date helpers:

- `price_parts(amount) -> table`
  - Returns `{ dollars = string, cents = string }`.
  - Example: `price_parts(12.99)` -> `{ dollars = "12", cents = "99" }`.
- `date_format(input, pattern) -> string`
  - Input must be `YYYY-MM-DD`.
  - Supports token replacement for `YYYY`, `MM`, and `DD`.
  - Example: `date_format("2026-05-15", "MM/DD/YYYY")` -> `"05/15/2026"`.

## `data` API

```lua
local v = data.getSlot("slot_name")
```

`data.getSlot` behavior:

- Reads top-level value from JSON object.
- Returns Lua `string`, `number`, `boolean`, or `nil`.
- Missing key returns `nil`.
- JSON arrays/objects fail hard with runtime error.

## `layer` API

`layer` is a strict userdata with mutable fields:

- `layer.visible: boolean`

Rules:

- Invalid `layer.visible` assignment fails render hard.

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
- `image` uses script override as asset alias (package alias or alias created by `loadimage`).
- `barcode` uses script override as barcode payload.

When `frame.value` is `nil` or empty string, renderer falls back to normal PSL evaluation.

## Error Policy

- Script compile/runtime/type errors fail render immediately.
- Errors include frame id and frame index context.
