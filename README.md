<p align="center">
  <img alt="Marmot - A freaky fast PDF rendering library." src="https://user-images.githubusercontent.com/15202776/225741238-31eaf8f8-c7a3-4135-a385-71fcb2f6f967.png">
</p>

<p align="center">
Generate PDF files so fast <a href="https://youtu.be/syNumVb2kUs?t=8" target="_blank">you'll (fr)eek</a>.
</p>

## Roadmap

- [ ] PDF Document basics
  - [ ] Create new document (memory)
  - [ ] Output documents
    - [ ] Save buffer to file
    - [ ] Output buffer (needed for API support)
- [ ] Pages
  - [ ] Create new page
  - [ ] Set page size (w, h)
- [ ] Rectangles
  - [ ] Create rectangle (w, h, x, y)
  - [ ] Fill colour
  - [ ] Stroke colour
  - [ ] Add to page
- [ ] Text
  - [ ] Create text (w, h, x, y)
  - [ ] Font colour
  - [ ] Line wrapping
  - [ ] Vertical alignment (top, middle, bottom)
  - [ ] Text alignment (left, center, right, justify)
  - [ ] Fitting modes (none, squeeze, stretch)
  - [ ] Custom font support
  - [ ] Add to page
- [ ] Images
  - [ ] Create image (w, h, x, y)
  - [ ] Encode image data
  - [ ] Compress image data
  - [ ] Add to page

> **Note**: roadmap may be incomplete. New features/use cases will be added during development.

## References

### Specification

- [PDF 2.0 Spec](https://www.iso.org/standard/75839.html)
- [PDF 1.7 Spec (free)](https://web.archive.org/web/20220226063926/https://www.adobe.com/content/dam/acom/en/devnet/pdf/pdfs/PDF32000_2008.pdf)

### Other Implementations

- [iText 7 (.NET)](https://github.com/itext/itext7-dotnet)
- [iText 7 (Java)](https://github.com/itext/itext7)
- [PDFKit (NodeJS)](https://pdfkit.org/)
- [JagPDF (C++)](https://github.com/jgresula/jagpdf)
