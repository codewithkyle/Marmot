# Marmot CLI Guide

Marmot renders dynamic PDFs from packaged `.psl` templates and JSON data.

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
4. Render a PDF.

Example:

```bash
cargo run -- pack test/test-1.psl demo
cargo run -- check demo.marmot data/test-1.json
cargo run -- render demo.marmot data/test-1.json --output out.pdf
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

Render a `.marmot` package into a PDF.

```bash
marmot render --output <output> <package> [data]
```

Example with data:

```bash
marmot render demo.marmot data/test-1.json --output out.pdf
```

Example without data:

```bash
marmot render demo.marmot --output out.pdf
```

Behavior:

- Loads and parses `template.psl` from package.
- If `[data]` is provided, parses JSON and validates slot types/required fields.
- Builds render context, including package font aliases.
- Renders to PDF at `--output`.

Notes:

- If template uses slot values in `draw`, rendering without `[data]` fails.
- Current renderer output format is PDF.

## `marmot pack`

Create a `.marmot` package from a template and optional files.

```bash
marmot pack [OPTIONS] <template> <name>
```

Options:

- `-a, --asset <PATH>`: include an asset file in archive at `assets/<filename>`
- `-f, --font <PATH>`: include a font file in archive at `fonts/<filename>`
- `-o, --output-dir <DIR>`: output directory (defaults to current directory)

Example:

```bash
marmot pack test/test-6.psl label -f fonts/Kablammo.ttf -o build
```

Creates: `build/label.marmot`

## Package Format

A `.marmot` file is a zip archive with at least:

- `template.psl`

When options are used, it can also contain:

- `fonts/<filename>` (from `--font`)
- `assets/<filename>` (from `--asset`)

Important:

- `pack` deduplicates archive paths and errors on duplicate filenames.
- `pack` currently refuses to overwrite an existing `.marmot` output file.

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
- Render output parent directory must already exist.
- `pack --output-dir` must exist and be a directory.
- `pack` output file extension must be `.marmot`.

## Troubleshooting

## `package must end with .marmot`

- Cause: `check`/`render` package path missing `.marmot` extension.
- Fix: use a valid `.marmot` package path.

## `package is missing template.psl`

- Cause: archive does not contain `template.psl`.
- Fix: rebuild package with `marmot pack`.

## `data does not match template slots`

- Cause: JSON is missing required slots or has wrong types.
- Fix: match your JSON fields to slot definitions in template.

## `output directory does not exist`

- Cause: render/pack output parent folder is missing.
- Fix: create the directory first.

## `duplicate font alias` or `duplicate package entry`

- Cause: duplicated font alias in template or repeated file names during packaging.
- Fix: make aliases and package filenames unique.

## What Is Implemented Today

- Commands: `check`, `render`, `pack`
- Input data format: JSON object
- Output format: PDF
- Packaged fonts: supported
- Packaged assets: can be packaged, but there is no draw-time asset/image operator yet
