# Design

## Behaviour

The image is square with side S read from the image object. The centre is x + (width − S) / 2 and y + (height − S) / 2, both truncated toward zero; when flags & 0x60 is 0x40, y gains one. The place's image word and whether the image was the global default are kept.

The mode is flags & 0xf0000. Mode 0x10000 draws through the scaled call with scale 1.0 and binds no ink. Every other mode binds an ink:

- style & 0x1000000 set: bit 0x10 binds slot 0xd8 with no colour; bit 0x40 binds 0xf4 with the raised colour; otherwise 0xe4 with the plain colour.
- style & 0x1000000 clear: flags & 0x11 binds 0xd8; bit 0x40 binds 0xf4; otherwise 0xe4; none of these passes an explicit colour.

The raised and plain colours are three bytes from the skin plus the one alpha byte. Style bits 0x70000000 select frame index (style >> 28) & 7; zero selects no frame.

## Open questions

The image object's type and the two draw helpers are not established. The plain and raised colour fields are named only by offset.

## Docs

This method is 788 bytes. `TGUIDrawSkin` is 40%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
