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

Example timings output (machine-dependent):

```
batch: jobs=16
batch complete: success=10000, failed=0, skipped=0
timings:
    prep:    44.869 ms
    process: 36.447 s
    total:   36.492 s
    render avg:   58.170 ms
    render min:   37.399 ms
    render max:   411.573 ms
    render p90:   75.744 ms
    render p95:   90.661 ms
    render p99:   109.099 ms
    render p99.9: 387.530 ms
    script avg:   0.083 ms
    script min:   0.044 ms
    script max:   4.088 ms
    draw avg:     11.026 ms
    draw min:     6.875 ms
    draw max:     370.979 ms
```
