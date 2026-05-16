# Marmot CLI Guide

Marmot renders dynamic PDFs and PNGs from packaged `.psl` templates and JSON data.

## Quick Start

Build the binary:

```bash
cargo build
```

Show top-level help:

```bash
cargo run -- --help
```

Basic workflow:

1. Create a template file, for example `template.psl`.
2. Package it into a `.marmot` archive.
3. Validate JSON data against template slots.
4. Render a PDF or PNG.
5. (Optional) Batch-render many PDFs/PNGs from JSONL records.

Example:

```bash
cargo run -- pack test/test-1.psl demo
cargo run -- check demo.marmot data/test-1.json
cargo run -- render demo.marmot data/test-1.json --output out.pdf
cargo run -- batch demo.marmot data/batch-10k.jsonl --output-dir out --output-name "{sku}.pdf" --timings
```

## Command Reference

## `marmot check`

Validate a `.marmot` package against a JSON data file.

```bash
marmot check <package> <data>
```

Example:

```bash
marmot check demo.marmot data/test-1.json
```

Behavior:

- Opens and unpacks the package.
- Reads and parses `template.psl` from the archive.
- Validates JSON against declared `slots`.
- Prints `OK` on success.
- Prints validation errors and exits with failure when data does not match.

## `marmot render`

Render a `.marmot` package into a PDF or PNG.

```bash
marmot render [--timings] [--output-type <pdf|png>] [--dpi <72-1200>] [--dither <type>] [--allow-host-assets] --output <output> <package> [data]
```

Example with data:

```bash
marmot render demo.marmot data/test-1.json --output out.pdf
```

Example without data:

```bash
marmot render demo.marmot --output out.pdf
```

Example with stage timings:

```bash
marmot render demo.marmot data/test-1.json --output out.pdf --timings
```

Example PNG with dithering:

```bash
marmot render demo.marmot data/test-1.json --output out.png --output-type png --dither floyd
```

Behavior:

- Loads and parses `template.psl` from package.
- If `[data]` is provided, parses JSON and validates slot types/required fields.
- Builds render context, including package font aliases.
- Loads package layer/frame scripts from `scripts/*.lua` when present.
- Allows `loadimage` host filesystem reads only with `--allow-host-assets`.
- Resolves relative `loadimage` host paths from current working directory.
- Renders to `--output` as PDF or PNG (`--output-type`, default `pdf`).
- Prints non-fatal render warnings to stderr (for example empty frame values).
- With `--timings`, prints elapsed time for `prep`, `render`, `script`, `draw`, and `total`.

Notes:

- If template uses slot values in `draw`, rendering without `[data]` fails.
- Template `layers begin ... end` and nested `draw -> layer -> frame` sections are required.
- `--dpi` applies to PNG output (default `300`, range `72..=1200`).
- `--dither` applies to PNG output and requires `remap.plt` in the package.
- `--allow-host-assets` enables runtime host filesystem image reads via PSL `loadimage`.
- Supported dither types: `floyd`, `atkinson`, `stucki`, `burkes`, `jarvis`, `sierra3`.
- `--timings` is intended for local profiling and benchmarking runs.

## `marmot pack`

Create a `.marmot` package from a template and optional files.

```bash
marmot pack [OPTIONS] <template> <name>
```

Options:

- `-a, --asset <PATH>`: include an asset file in archive at `assets/<filename>`
- `-f, --font <PATH>`: include a font file in archive at `fonts/<filename>`
- `-s, --script <PATH>`: include a Lua script file in archive at `scripts/<filename>`
- `-o, --output-dir <DIR>`: output directory (defaults to current directory)
- `--remap <PATH>`: include a remap palette file in archive as `remap.plt` (used by PNG dithering)

Example:

```bash
marmot pack test/test-6.psl label -f fonts/Kablammo.ttf -s scripts/FRAME_1.lua -o build
```

Creates: `build/label.marmot`

## `marmot batch`

Render many PDFs or PNGs from one package and a JSONL records file.

```bash
marmot batch [OPTIONS] <package> <records> --output-dir <dir> --output-name <template>
```

Options:

- `--output-dir <DIR>`: destination directory for generated files (created if missing)
- `--output-name <TEMPLATE>`: filename template supporting `{index}` and top-level JSON fields (e.g. `{sku}`, `{id}`)
- `--output-type <TYPE>`: output format, `pdf` or `png` (default `pdf`)
- `--dpi <NUMBER>`: PNG DPI (`72..=1200`, default `300`)
- `--dither <TYPE>`: PNG dither algorithm; requires package `remap.plt`
- `--allow-host-assets`: enable runtime host filesystem image reads via PSL `loadimage`
- `-j, --jobs <N>`: worker count (`0` = auto-detect CPU parallelism)
- `--trust-data`: skip upfront per-record slot validation in batch mode
- `--timings`: print stage timings and per-record render latency distribution

Supported dither types: `floyd`, `atkinson`, `stucki`, `burkes`, `jarvis`, `sierra3`.

Examples:

```bash
marmot batch demo.marmot data/batch-10k.jsonl --output-dir out --output-name "{sku}.pdf"
marmot batch demo.marmot data/batch-10k.jsonl --output-dir out --output-name "{index}-{sku}-{buy_qty}-{get_qty}.pdf" -j 16 --timings
marmot batch demo.marmot data/batch-10k.jsonl --output-dir out --output-name "{sku}.pdf" --trust-data --timings
```

Behavior:

- Reads `<records>` as JSON Lines (one JSON object per line).
- Ignores blank lines.
- Uses a worker pool to render records in parallel.
- Resolves relative `loadimage` host paths from current working directory.
- Produces one PDF/PNG per successful record.
- Prints per-record non-fatal render warnings to stderr.
- Prints `success`, `failed`, and `skipped` counts at completion.

Batch timing output (`--timings`) includes:

- `prep`, `process`, and `total` wall-clock stages.
- Render latency stats across rendered records: `avg`, `min`, `max`, `p90`, `p95`, `p99`, `p99.9`.
- Script and draw stats across rendered records: `script avg/min/max`, `draw avg/min/max`.

Output name template notes:

- `{index}` uses 1-based input line number.
- Any other placeholder like `{sku}` reads a top-level field from each JSONL record.
- You can combine many fields, e.g. `{index}-{sku}-{buy_qty}-{get_qty}.pdf`.
- Referenced fields must be string/number/bool.
- Output names are sanitized to avoid invalid filesystem characters.
- Path `..` segments are rejected.

## Package Format

A `.marmot` file is a zip archive with at least:

- `template.psl`

When options are used, it can also contain:

- `fonts/<filename>` (from `--font`)
- `assets/<filename>` (from `--asset`)
- `scripts/<filename>` (from `--script`)
- `remap.plt` (from `--remap`)

Important:

- `pack` deduplicates archive paths and errors on duplicate filenames.

## Font Resolution at Render Time

At render time:

1. Font aliases from `fonts begin ... end` are resolved to package files.
2. Font files are registered with fontconfig and family names are extracted.
3. `font` operator selects either:
   - packaged font alias (preferred when alias exists), or
   - system font by name (fallback when alias is not declared).

## Path and Argument Validation Rules

Common validation behavior:

- Package path must exist, be a file, and end with `.marmot`.
- Data path (for `check` or `render` with data) must exist and be a file.
- Records path (for `batch`) must exist and be a file.
- Render output parent directory is created if missing.
- Batch output directory is created if missing.
- `pack --output-dir` is created if missing.
- `pack` output file extension must be `.marmot`.

## Troubleshooting

## `package must end with .marmot`

- Cause: `check`/`render`/`batch` package path missing `.marmot` extension.
- Fix: use a valid `.marmot` package path.

## `package is missing template.psl`

- Cause: archive does not contain `template.psl`.
- Fix: rebuild package with `marmot pack`.

## `data does not match template slots`

- Cause: JSON is missing required slots or has wrong types.
- Fix: match your JSON fields to slot definitions in template.

## `failed to create directory`

- Cause: output folder cannot be created (permissions, invalid path, read-only filesystem).
- Fix: choose writable path or adjust permissions.

## `duplicate font alias` or `duplicate package entry`

- Cause: duplicated font alias in template or repeated file names during packaging.
- Fix: make aliases and package filenames unique.

## `--dither requires remap.plt in package`

- Cause: `--dither` used with `render` or `batch`, but package does not include `remap.plt`.
- Fix: rebuild package with `marmot pack ... --remap <palette-file>`.

## `unknown script file` or `invalid script file extension`

- Cause: file in package `scripts/` does not map to declared layer/frame id, or script file is not `.lua`.
- Fix: rename script to `<layer_id>.lua` or `<frame_id>.lua` and keep only Lua files in `scripts/`.

## `ScriptRuntime` / script failed for layer or frame

- Cause: Lua runtime error or invalid assignment (`layer.visible`, `frame.visible`, `frame.value`).
- Fix: check script line and types; ensure `layer.visible`/`frame.visible` are boolean and `frame.value` is string or `nil`.

## `record missing field '<name>' required by output template`

- Cause: `--output-name` references a JSON field that is missing in a record.
- Fix: add the field to each record or adjust the template (for example use `{index}` or an existing key like `{sku}`).

## `batch produced no outputs`

- Cause: all records failed before/during render.
- Fix: inspect per-line errors, validate sample records with `marmot check`, then retry.

## What Is Implemented Today

- Commands: `check`, `render`, `pack`, `batch`
- Input data formats: JSON object (`check`/`render`) and JSONL records (`batch`)
- Output formats: PDF, PNG
- Packaged fonts: supported
- Packaged image assets + draw-time image operator: supported
- Packaged layer/frame scripting (`scripts/*.lua`): supported
