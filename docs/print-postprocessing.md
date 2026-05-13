# Print Post-Processing Recommendations

This guide covers recommended downstream steps to turn Marmot PDF output into print-ready deliverables.

## Goal

Pick one target standard, then enforce it for every job:

- `PDF/X-4` for modern RIP workflows (live transparency + ICC color management)
- `PDF/X-1a` for legacy/strict CMYK workflows (more conservative compatibility)

## Recommended Pipeline

1. Render variable PDFs with Marmot.
2. Convert/conform to target print profile.
3. Preflight and fail nonconforming output.
4. Impose and send to RIP.

## Tooling

### Conversion and normalization

- Ghostscript (open source): color conversion, PDF normalization, and PDF/X-oriented output workflows.
  - https://ghostscript.com/
  - https://ghostscript.readthedocs.io/

### Validation and preflight

- [callas pdfToolbox](https://www.callassoftware.com/en/products/pdftoolbox): production-grade preflight/fixups for print workflows.
- [Adobe Acrobat Pro Preflight](https://helpx.adobe.com/acrobat/using/preflight-pdfs-acrobat-pro.html): broad prepress checks and PDF/X validation.
- [Enfocus PitStop Pro](https://www.enfocus.com/en/pitstop-pro): print preflight and correction workflows.
- [veraPDF](https://verapdf.org/) (open source): strong PDF/A validator (useful for structural checks, not print-specific replacement for prepress suites).

### Optional PDF inspection and repair helpers

- [qpdf](https://qpdf.sourceforge.io/) (open source): inspect/fix low-level PDF structure issues, linearization, encryption handling.
- [pdfcpu](https://pdfcpu.io/) (open source): inspect, optimize, validate, and manipulate PDF files.

## Color management notes

- CMYK values converted to RGB inside renderer are not equivalent to true press CMYK separations.
- If strict brand color or ink control is required, validate against printer ICC/profile and RIP behavior.
- Keep one approved profile set per press condition and lock your pipeline to it.

## Operational guidance

- Define one job contract per customer/workflow (`PDF/X-4` or `PDF/X-1a`, profile, trim/bleed rules).
- Run automated preflight gate before imposition/RIP.
- Keep small proof subset and sign-off loop before full run.
- Version-control conversion/preflight settings so jobs are reproducible.
