# Marmot Tutorial: Render Your First PDF

This tutorial gives copy/paste steps to render one realistic label from a `.psl` template and one JSON record.

## Prerequisites

- Run commands from repository root.

## Build and Render

Copy/paste this into terminal:

```bash
cargo run -- pack ./docs/tutorial/tutorial.psl tutorial -f test/fonts/Kablammo.ttf -a test/images/walmart.png -o ./out
cargo run -- check ./out/tutorial.marmot ./docs/tutorial/tutorial.json
cargo run -- render ./out/tutorial.marmot ./docs/tutorial/tutorial.json --output ./out/tutorial.pdf
```

Expected result:

- `check` prints `OK`.
- Render output written to `./out/tutorial.pdf`.

