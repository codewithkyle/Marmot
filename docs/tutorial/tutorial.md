# Marmot Tutorial: Render Your First PDF

This tutorial gives copy/paste steps to render one realistic label from a `.psl` template and one JSON record.

## Prerequisites

- Run commands from repository root.

## Build and Render

Copy/paste this into terminal:

```bash
cargo run -- pack ./docs/tutorial/tutorial.psl tutorial -f test/fonts/Kablammo.ttf -a test/images/sprout-basket.png -o ./out -s ./docs/tutorial/FRAME_QR_CODES.lua -s ./docs/tutorial/FRAME_LOGO.lua -a test/images/save-5.png
cargo run -- check ./out/tutorial.marmot ./docs/tutorial/tutorial.json
cargo run -- render ./out/tutorial.marmot ./docs/tutorial/tutorial.json --output ./out/tutorial.pdf
```

Expected result:

- `check` prints `OK`.
- Render output written to `./out/tutorial.pdf`.

## Batch Render

After running the `pack` command above you can batch render 10,000 PDFs using the `batch` command:

```bash
cargo run -- batch ./out/tutorial.marmot ./docs/tutorial/tutorial-10k.jsonl --output-dir ./out --output-name "{sku}.pdf" --timings
```

Example timings output:

```
batch: jobs=16
batch complete: success=10000, failed=0, skipped=0
timings:
    prep:    40.366 ms
    process: 24.043 s
    total:   24.083 s
    render avg:   38.360 ms
    render min:   23.355 ms
    render max:   373.105 ms
    render p90:   43.069 ms
    render p95:   50.459 ms
    render p99:   75.215 ms
    render p99.9: 348.927 ms
```
