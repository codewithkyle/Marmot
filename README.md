<p align="center">
  <img width="1280" height="640" alt="378975058-d348c592-3bb4-4fd0-b8e4-3b2c6718ffeb" src="https://github.com/user-attachments/assets/33748d50-e4e0-4270-9212-607ed210a1b7" />
</p>

<p align="center">
Rendering dynamic PDFs so fast <a href="https://youtu.be/syNumVb2kUs?t=8" target="_blank">you'll (fr)eek</a>.
</p>

## Introduction

Marmot is a PostScript-inspired template renderer for generating small dynamic PDFs and images from structured data.

The initial goal is to render label-sized PDFs from Marmot templates and JSON/JSONL data, with a focus on deterministic batch rendering.

## PSL (PostScript-Like) Template Language

```
%!PSL 0.1

page 612 792

slots begin
  product_name string required
  base_price string required
  sale_price string required
  buy int required
  get int required
end

fonts begin
  helvetica "fonts/Helvetica.ttf"
  helvetica_bold "fonts/Helvetica-Bold.ttf"
end

assets begin
  logo image "assets/logo.png"
  badge image "assets/logo.png"
end

draw begin
  % Background and border
  1 1 1 rgb
  0 0 612 792 rect fill

  1 0 0 rgb
  72 72 468 648 rect stroke

  % Embedded image
  logo 420 40 120 60 image contain

  % Product name
  $(helvetica_bold) font
  28 fontsize
  center align
  middle valign
  0 0 0 1 cmyk
  $(product_name) 72 100 468 80 textbox

  % Offer
  helvetica font
  64 fontsize
  (BUY ) $(buy) ( GET ) $(get) 4 concat 72 240 468 100 textbox % output: "BUY 2 GET 1"

  % Price
  helvetica_bold font
  96 fontsize
  0.5 grey
  $(sale_price) 72 380 468 130 textbox
end
```

## References

- [PostScript Language Reference - 3rd Edition](https://drive.google.com/file/d/1MKZm12NrNdp2CyIV_yLKQnurBlvXn1Ji/view?usp=sharing)
- [What is a Lexer (video)](https://www.youtube.com/watch?v=BI3K-ME3L74)
- [Tokenize Text From Scratch in Rust (video)](https://www.youtube.com/watch?v=64nGSSQ3HSE)
- [Writing a Simple Parser in Rust](https://adriann.github.io/rust_parser.html)
- [Intro to Cairo Graphics in Rust](https://medium.com/@bit101/intro-to-cairo-graphics-in-rust-35470a6aed86)
