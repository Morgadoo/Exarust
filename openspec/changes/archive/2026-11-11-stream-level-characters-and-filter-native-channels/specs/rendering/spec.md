## ADDED Requirements

### Requirement: Mip chains in the word's channels
A generated mip chain SHALL filter the channels of the image's pixel word (one for `0x2008` and `0x4200`, two for `0x6208` and `0x8400`, three for `0xC600`, four otherwise) when the RGBA8 base is laid out as that word's decode makes it, and spread the filtered channels back as the decode does; it SHALL filter all four when the base is laid out otherwise or the image is renormalised.

#### Scenario: A grey image
- **WHEN** an 8 x 4 grey image (word `0x4200`, wrapped, squared space) builds its chain
- **THEN** levels 1 to 3 hold the greys 168 152 140 138 147 128 144 153, 149 144 and 147, with opaque alpha, the bytes the four-channel path gives

#### Scenario: A grey word whose colours differ
- **WHEN** a base of word `0x4200` has a texel whose red and green differ
- **THEN** all four channels are filtered
