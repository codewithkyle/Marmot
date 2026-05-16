# PSL (`.psl`) Language Reference

PSL is a PostScript-like template language used by Marmot.

This document describes the syntax and semantics currently implemented.

## Minimal Valid Template

```psl
%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

layers begin
  layer 1 LAYER_MAIN begin
    1 FRAME_1
  end
end

draw begin
  frame 1 begin
  end

  layer 1 begin
  frame 1 begin
  end
  end
end
```

## File Structure and Order

Current parser expects this order:

1. Header comment: `%!PSL <version>`
2. `page <width> <height>`
3. Optional `slots begin ... end`
4. Optional `fonts begin ... end`
5. Optional `assets begin ... end`
6. Required `frames begin ... end`
7. Optional `layers begin ... end`
8. Required `draw begin ... end`

Notes:

- `slots`, `fonts`, and `assets` are optional blocks.
- `frames` and `draw` are required blocks.
- `slots`, `fonts`, `assets`, and `layers` are optional blocks.
- Block order is fixed: `slots` -> `fonts` -> `assets` -> `frames` -> `layers` -> `draw`.

## Lexical Tokens

## Comments

- Start with `%` and continue to end of line.
- Leading/trailing whitespace in comment text is trimmed by lexer.

## Strings

Two forms are supported:

- Parenthesized: `(Hello world)`
- Double-quoted: `"fonts/Helvetica.ttf"`

Escape sequences inside strings:

- `\(`, `\)`, `\\`, `\n`, `\t`, `\r`
- Unknown escapes are passed through (for example `\z` becomes `z`).
- Strings cannot span newlines.

## Words

- Start: ASCII letter or `_`
- Continue: ASCII letters, digits, or `_`

Examples:

- valid: `page`, `product_name`, `_x`, `name1`
- invalid: `-name`, `1name`, `name-with-dash`

## Numbers

- Unsigned decimal literals only.
- Supported examples: `12`, `12.5`, `0.25`
- Rejected examples: `-1`, `.5`, `1e3`, `12.`

## Slot Variables

Form:

```psl
$(slot_name)
```

Rules:

- Must use the same naming rules as words.
- Must not be empty.
- Must not contain invalid characters.

## Header

First token must be a comment with prefix:

```psl
%!PSL 0.1
```

If the first comment is not prefixed with `!PSL `, parsing fails.

## Page

Syntax:

```psl
page <width> <height>
```

- `width` and `height` are numeric literals.

## Slots Block

Syntax:

```psl
slots begin
  <name> <type> [required]
  ...
end
```

Slot types:

- `string`
- `int`
- `decimal`

Examples:

```psl
slots begin
  product_name string required
  sale_price decimal required
  buy int
end
```

Validation semantics:

- Data must be a JSON object.
- `required` slots must be present.
- `string` expects JSON string.
- `int` expects integer-valued JSON number.
- `decimal` expects any JSON number.

## Fonts Block

Syntax:

```psl
fonts begin
  <alias> "<package-relative-path>"
  ...
end
```

Example:

```psl
fonts begin
  helvetica_bold "fonts/Helvetica-Bold.ttf"
end
```

Rules:

- Alias must be a word token.
- Path must be a string token.
- Paths are resolved inside the `.marmot` package.
- Duplicate aliases are rejected at render-context build time.

## Assets Block

Syntax:

```psl
assets begin
  <alias> image "<package-relative-path>"
  ...
end
```

Example:

```psl
assets begin
  logo image "assets/logo.png"
end
```

Rules:

- Alias must be a word token.
- Asset type is currently `image`.
- Path must be a string token.
- Paths are resolved inside the `.marmot` package.
- Duplicate aliases are rejected at render-context build time.

## Frames Block

Syntax:

```psl
frames begin
  <u32> <id>
  ...
end
```

Rules:

- Frame `<u32>` must be a non-negative integer in `u32` range.
- Frame `<id>` must be a word token.

## Layers Block

Syntax:

```psl
layers begin
  layer <u32> <id> begin
    <u32> <frame_id>
    ...
  end
end
```

Rules (optional block):

- Layer `<u32>` must be a non-negative integer in `u32` range.
- Layer `<id>` must be a word token.
- Frame `<u32>` must be a non-negative integer in `u32` range.
- Frame `<frame_id>` must be a word token.
- Layer frame entries are metadata references to declared frames.
- Layer frame indices are referenced by `draw` layer blocks.

## Draw Block

Syntax:

```psl
draw begin
  frame <u32> begin
    ...operators...
  end

  layer <u32> begin
    frame <u32> begin
      ...operators...
    end
  end
end
```

Rules:

- Draw block entries may be either:
  - `frame <u32> begin ... end` (standalone/shared frame)
  - `layer <u32> begin ... end` containing one or more frame blocks
- Referenced frame index must exist in the `frames` block.
- Referenced layer index must exist in the `layers` block.
- Frames inside a draw layer must belong to that layer declaration.
- Each frame section has its own stack/path validation.

## Scripting Integration

PSL itself does not embed Lua syntax. Scripts are external package files.

- Script path: `scripts/<script_id>.lua` inside `.marmot` package.
- `<script_id>` may be either a layer id from `layers begin ... end` or a frame id from `frames begin ... end`.
- Missing scripts are valid (no-op).
- Unknown script file (no matching layer/frame id) fails context build.
- A layer id and frame id cannot share the same script id.

At render time, scripts can mutate runtime properties:

- `layer.visible: boolean`

- `frame.visible: boolean`
- `frame.value: string | nil`

Override behavior:

- Non-empty `frame.value` overrides value-bearing ops in that frame:
  - `textbox` text
  - `image` asset alias
  - `barcode` payload
- `frame.value = nil` or empty string falls back to normal PSL draw-op evaluation.

For full API and runtime details, see [`docs/scripting.md`](docs/scripting.md).

## Frame/Layer Blocks in `draw`

`draw` is always frame-scoped. Layer blocks are optional grouping wrappers.

Valid shape:

```psl
draw begin
  frame 3 begin
    0 0 0 rgb
    20 20 100 20 rect fill
  end

  layer 1 begin
  frame 1 begin
    1 0 0 rgb
    10 10 100 40 rect fill
  end
  end
end
```

Invalid shape (operators at top level of `draw`):

```psl
draw begin
  1 0 0 rgb
  10 10 100 40 rect fill
end
```

In other words, `draw begin ... end` contains one or more `frame <u32> begin ... end` and/or `layer <u32> begin ... end` blocks, and the actual draw operators always live inside frame blocks.

The draw language is stack-based.

Literals push onto stack:

- number literal -> number stack value
- string literal -> text stack value
- `$(slot)` -> typed stack value based on declared slot type

## Supported Operators

## Color and stroke

- `rgb` consumes `r g b` (numbers)
- `cmyk` consumes `c m y k` (numbers)
- `strokewidth` consumes `width` (number), must be `> 0` for literal values

Color value behavior:

- Parser does not enforce `0..=1` bounds for `rgb`/`cmyk` values.
- Renderer clamps final RGB channels to `0..=1`.

Example:

```psl
1 0 0 rgb
0 1 1 0 cmyk
2 strokewidth
```

## Geometry paths and paint

- `line` consumes `x1 y1 x2 y2`
- `rect` consumes `x y width height` (`width` and `height` must be `> 0` for literals)
- `stroke` paints current path
- `fill` paints current path, but only valid for `rect`

State rules:

- You cannot start a new path while a prior path is unpainted.
- `stroke`/`fill` require a current path.
- `fill` on a line path is an error.

Example:

```psl
0 0 100 100 line stroke
10 10 80 40 rect fill
```

## Text styling and layout

- `font` consumes one text value (font alias or system font name)
- `fontsize` consumes one number (`> 0` for literals)
- `left align`, `center align`, `right align`
- `top valign`, `middle valign`, `bottom valign`
- `word wrap`, `char wrap`, `none wrap`
- `fixed textfit`, `shrink textfit`, `grow textfit`, `fit textfit`
- `textfitmin` consumes one number (`> 0` for literals)
- `textfitmax` consumes one number (`> 0` for literals)

Example:

```psl
"Noto Sans Mono" font
14 fontsize
center align
middle valign
word wrap
grow textfit
8 textfitmin
36 textfitmax
```

## Text drawing

- `textbox` consumes five values:
  - `text x y width height`
  - `width` and `height` must be `> 0` for literals
- `concat` consumes one number `count`, then consumes `count` text values and pushes one combined text value
  - `count` must be a literal non-negative integer
  - `count` cannot come from a slot
  - nested `concat` values are rejected by parser
- `uppercase` consumes one text value and pushes uppercase text
- `lowercase` consumes one text value and pushes lowercase text
- `capitalize` consumes one text value and pushes capitalized text (first grapheme uppercased, remainder lowercased)
- `titlecase` consumes one text value and pushes title-cased text

Example:

```psl
$(product_name) 20 40 260 40 textbox
```

Additional examples:

```psl
(BUY ) $(B) ( GET ) $(G) 4 concat 20 40 260 40 textbox

$(product_name) uppercase 20 40 260 40 textbox
$(product_name) lowercase 20 40 260 40 textbox
$(product_name) capitalize 20 40 260 40 textbox
$(product_name) titlecase 20 40 260 40 textbox
```

## Image drawing

- `loadimage` consumes two values:
  - `path alias`
  - `path` is a text value resolved at render time
  - `alias` is a text value used by later `image` operators
  - Requires CLI flag `--allow-host-assets` for host filesystem access
  - Relative paths are resolved from current working directory where `marmot render`/`marmot batch` runs
- `image` consumes five values:
  - `asset x y width height`
  - `asset` is a text value (literal or string slot)
  - `width` and `height` must be `> 0` for literals
- `contain imagefit`, `cover imagefit`, `stretch imagefit` set image fitting mode

Example:

```psl
"./logos/sprout-basket.png" "customer_logo" loadimage
contain imagefit
(customer_logo) 20 20 120 60 image
```

## Barcode drawing

- Symbology words push a barcode value onto the stack:
  - `c39`, `c128a`, `c128b`, `c128c`, `upca`, `ean13`, `ean8`, `msi`, `qr`, `datamatrix`
- `barcode` consumes six values:
  - `value symbology x y width height`
  - `value` is a text value (literal, string slot, or transformed text)
  - `symbology` must be one of the words above
  - `width` and `height` must be `> 0` for literals

Render behavior notes:

- 1D codes (`c39`, `c128a/b/c`, `upca`, `ean13`, `ean8`, `msi`) are drawn as vector bars.
- `upca`, `ean13`, and `ean8` guard bars are extended by about `5X` (module widths).
- `qr` is encoded with error correction level `M` and a 4-module quiet zone.
- `datamatrix` is encoded with Data Matrix symbols (can be rectangular) and a 1-module quiet zone.

Example:

```psl
$(sku) c39 20 20 200 48 barcode
$(gtin) ean13 20 80 200 72 barcode
$(url) qr 240 20 80 80 barcode
$(serial) datamatrix 340 20 80 80 barcode
```

## Runtime Defaults

Initial render state defaults:

- font: `Sans`
- font size: `12`
- text alignment: `left`
- vertical alignment: `top`
- line break mode: `word`
- text fit mode: `fixed`
- text fit min/max: `4` and `96`
- image fit mode: `contain`
- text clipping inside textbox: enabled

## Slot Use in `draw`

When `$(slot)` appears in a frame draw block:

- Slot must be declared in `slots` block.
- Slot type controls how parser interprets it:
  - `string` -> text value
  - `int`/`decimal` -> number value
- During render, missing data or wrong JSON type causes render errors.

## Error Conditions (High-Level)

## Lex errors

- Unknown characters
- Unterminated strings or slot variables
- Invalid numeric literals
- Invalid slot syntax

## Parse errors

- Missing or invalid header
- Missing expected keywords (`begin`, `end`, etc.)
- Unexpected EOF in blocks
- Stack underflow or leftover stack values
- Unknown frame index references in `draw`
- Unknown slot references in `draw`
- Path state violations (`stroke`/`fill` misuse)
- Invalid literal operand constraints

## Validation errors

- Data is not a JSON object
- Missing required slot
- Wrong JSON type for slot

## Render errors

- Missing data for slot resolution
- Missing slot field in JSON
- Invalid text or number JSON value for slot
- Missing asset alias
- Wrong asset type
- Invalid image geometry
- Image decode/format issues
- Invalid barcode geometry
- Barcode encode/data validation failures
- Cairo rendering errors

## Full Example

```psl
%!PSL 0.1
page 300 120

slots begin
  product_name string required
end

fonts begin
  kablammo "fonts/Kablammo.ttf"
end

layers begin
  layer 1 LAYER_MAIN begin
    1 FRAME_BASE
    2 FRAME_TITLE
  end
end

draw begin
  layer 1 begin
  frame 1 begin
    1 1 1 rgb
    0 0 300 120 rect fill

    1 0 0 rgb
    20 40 260 40 rect fill
  end

  frame 2 begin
    0 0 0 rgb
    14 fontsize
    center align
    middle valign
    word wrap
    grow textfit
    (kablammo) font
    $(product_name) 20 40 260 40 textbox
  end
  end
end
```
