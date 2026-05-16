# Marmot Tutorial: Render Your First PDF and PNG

This tutorial gives copy/paste steps to render one realistic label from a `.psl` template and one JSON record.

## Prerequisites

- Run commands from repository root.

## Build and Render

Copy/paste this into terminal:

```bash
cargo run -- pack ./docs/tutorial/tutorial.psl tutorial -f test/fonts/Kablammo.ttf -a test/images/sprout-basket.png -a test/images/sprout-basket-alt.png -a test/images/save-5.png -a test/images/super-save.png -o ./out -s ./docs/tutorial/LAYER_DEFAULT_THEME.lua -s ./docs/tutorial/LAYER_SUPER_THEME.lua -s ./docs/tutorial/FRAME_BASE.lua -s ./docs/tutorial/FRAME_PRICE_TEXT.lua -s ./docs/tutorial/FRAME_PRICE_SALE.lua -s ./docs/tutorial/FRAME_LOGO_DEFAULT.lua -s ./docs/tutorial/FRAME_LOGO_SUPER.lua -s ./docs/tutorial/FRAME_BADGE_DEFAULT.lua -s ./docs/tutorial/FRAME_BADGE_SUPER.lua -s ./docs/tutorial/FRAME_QR_CODES.lua --remap ./docs/tutorial/remap.plt
cargo run -- check ./out/tutorial.marmot ./docs/tutorial/tutorial.json
cargo run -- render ./out/tutorial.marmot ./docs/tutorial/tutorial.json --output ./out/tutorial.pdf
cargo run -- render ./out/tutorial.marmot ./docs/tutorial/tutorial.json --output ./out/tutorial.png --output-type png --dither atkinson
```

Expected result:

- `check` prints `OK`.
- Render output written to `./out/tutorial.pdf`.
- Render output written to `./out/tutorial.png`.

Notes:

- Theme selection is layer-driven:
  - `LAYER_DEFAULT_THEME` renders when `regular_price * get_qty < 25.0`
  - `LAYER_SUPER_THEME` renders when `regular_price * get_qty >= 25.0`
  - this drives logo variant and badge family in one place
- Base/background and price colors are frame-script-driven with runtime overrides:
  - `FRAME_BASE` sets `frame.fill_color`, `frame.stroke_color`, and `frame.stroke_width`
  - `FRAME_PRICE_TEXT` and `FRAME_PRICE_SALE` set `frame.text_color`
- Shared frames (`FRAME_HEADER`, `FRAME_BAR_CODES`, `FRAME_QR_CODES`) are drawn directly in `draw` outside layer blocks.
- QR/logo behavior is frame-script-driven inside each theme:
  - if `promo_url` is empty: theme logo frame is visible and QR frame is hidden
  - if `promo_url` is non-empty: theme logo frame is hidden and QR frame is visible
- Badge behavior is frame-script-driven:
  - default theme badge (`save_5`) requires `regular_price >= 5.0`
  - super theme badge (`super_save`) is explicitly script-gated to super mode
- `--remap` on `pack` stores the palette in the package as `remap.plt`.
- `--dither` during `render` or `batch` requires `remap.plt` in the package.

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
    prep:    44.281 ms
    process: 32.475 s
    total:   32.519 s
    render avg:   51.870 ms
    render min:   34.116 ms
    render max:   400.833 ms
    render p90:   54.382 ms
    render p95:   56.474 ms
    render p99:   72.257 ms
    render p99.9: 390.030 ms
    script avg:   0.068 ms
    script min:   0.040 ms
    script max:   3.262 ms
    draw avg:     9.886 ms
    draw min:     6.860 ms
    draw max:     361.151 ms
```
