<p align="center">
  <img width="1280" height="640" alt="378975058-d348c592-3bb4-4fd0-b8e4-3b2c6718ffeb" src="https://github.com/user-attachments/assets/33748d50-e4e0-4270-9212-607ed210a1b7" />
</p>

<p align="center">
Rendering dynamic PDFs so fast <a href="https://youtu.be/syNumVb2kUs?t=8" target="_blank">you'll (fr)eek</a>.
</p>

## Introduction

Marmot is a PostScript-inspired template renderer for generating small dynamic PDFs and images from structured data.

The initial goal is to render label-sized PDFs from Marmot templates and JSON/JSONL data, with a focus on deterministic batch rendering.

## Documentation

- [CLI usage](docs/cli.md)
- [PSL language reference](docs/psl.md)
- [Scripting guide](docs/scripting.md)
- [Future ideas](docs/drafts)
- [Print post-processing](docs/print-postprocessing.md)

## PSL (PostScript-Like) Template Language

Example below uses currently supported syntax. For full operator details, see [PSL syntax documentation](docs/psl.md).

```psl
%!PSL 0.1

page 432 288

slots begin
  sku string required
  product_name string required
  sale_price decimal required
  regular_price decimal required
  buy_qty int required
  get_qty int required
  promo_url string required
end

fonts begin
  kablammo "fonts/Kablammo.ttf"
end

assets begin
  logo image "assets/sprout-basket.png"
end

frames begin
  1 FRAME_BASE
  2 FRAME_HEADER
  3 FRAME_OFFER
  4 FRAME_PRICE
  5 FRAME_LOGO
  6 FRAME_CODES
end

layers begin
  layer 1 LAYER_MAIN begin
    1 FRAME_BASE
    2 FRAME_HEADER
    3 FRAME_OFFER
    4 FRAME_PRICE
    5 FRAME_LOGO
    6 FRAME_CODES
  end
end

draw begin
  layer 1 begin
  frame 1 begin
    % Card background + border
    1 1 1 rgb
    0 0 432 288 rect fill

    0 0 0 0.08 cmyk
    6 strokewidth
    8 8 416 272 rect stroke
  end

  frame 2 begin
    % Header band
    0.92 0.07 0.16 rgb
    20 18 392 54 rect fill

    (kablammo) font
    22 fontsize
    center align
    middle valign
    1 1 1 rgb
    $(product_name) titlecase 28 24 376 40 textbox
  end

  frame 3 begin
    % Offer callout
    (kablammo) font
    32 fontsize
    left align
    middle valign
    0.1 0.1 0.1 rgb
    (BUY ) $(buy_qty) ( GET ) $(get_qty) 4 concat 26 88 250 44 textbox
  end

  frame 4 begin
    % Price stack
    (kablammo) font
    58 fontsize
    left align
    top valign
    fit textfit
    0.92 0.07 0.16 rgb
    ($) $(sale_price) 2 concat 26 136 220 74 textbox

    (kablammo) font
    14 fontsize
    left align
    top valign
    fit textfit
    0 0 0 rgb
    (sans) font
    (Reg $) $(regular_price) 2 concat 30 215 180 10 textbox
  end

  frame 5 begin
    % Brand image
    contain imagefit
    (logo) 285 90 110 110 image
  end

  frame 6 begin
    % Machine-readable codes
    0 0 0 rgb
    $(promo_url) qr 355 215 60 60 barcode
    $(sku) c128b 26 240 214 28 barcode
  end
  end
end
```
> **Wanna see what this looks like?** [Try our tutorial](docs/tutorial/tutorial.md).

## References

- [PostScript Language Reference - 3rd Edition](https://drive.google.com/file/d/1MKZm12NrNdp2CyIV_yLKQnurBlvXn1Ji/view?usp=sharing)
- [What is a Lexer (video)](https://www.youtube.com/watch?v=BI3K-ME3L74)
- [Tokenize Text From Scratch in Rust (video)](https://www.youtube.com/watch?v=64nGSSQ3HSE)
- [Writing a Simple Parser in Rust](https://adriann.github.io/rust_parser.html)
- [Intro to Cairo Graphics in Rust](https://medium.com/@bit101/intro-to-cairo-graphics-in-rust-35470a6aed86)
