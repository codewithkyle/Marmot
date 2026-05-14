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

draw begin
  frame 1 begin
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
7. Required `draw begin ... end`

Notes:

- `slots`, `fonts`, and `assets` are optional blocks.
- `frames` and `draw` are required blocks.
- Block order is fixed: `slots` -> `fonts` -> `assets` -> `frames` -> `draw`.

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

- `<u32>` must be a non-negative integer in `u32` range.
- `<id>` must be a word token.
- Frame indices are referenced by `draw` frame blocks.

## Draw Block

Syntax:

```psl
draw begin
  frame <u32> begin
    ...operators...
  end
  ...
end
```

Rules:

- Every draw block entry must be a `frame <u32> begin ... end` section.
- Referenced frame index must exist in the `frames` block.
- Each frame section has its own stack/path validation.

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

- `image` consumes five values:
  - `asset x y width height`
  - `asset` is a text value (literal or string slot)
  - `width` and `height` must be `> 0` for literals
- `contain imagefit`, `cover imagefit`, `stretch imagefit` set image fitting mode

Example:

```psl
contain imagefit
(logo) 20 20 120 60 image
```

## Barcode drawing

- Symbology words push a barcode value onto the stack:
  - `c39`, `c128a`, `c128b`, `c128c`, `upca`, `ean13`, `ean8`, `qr`, `datamatrix`
- `barcode` consumes six values:
  - `value symbology x y width height`
  - `value` is a text value (literal, string slot, or transformed text)
  - `symbology` must be one of the words above
  - `width` and `height` must be `> 0` for literals

Render behavior notes:

- 1D codes (`c39`, `c128a/b/c`, `upca`, `ean13`, `ean8`) are drawn as vector bars.
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

frames begin
  1 FRAME_BASE
  2 FRAME_TITLE
end

draw begin
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
```
