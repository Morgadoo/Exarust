## ADDED Requirements

### Requirement: Shipped item sentences
Every shipped item whose card builds a sentence SHALL get one that follows the description rules with the installed phrase tables: an in-range quality phrase (15 made rows, or 8 high rows for a dash-marked weapon), the name and at most one in-range condition phrase (flawless then perfect, or the 6 great rows), with an article when unbracketed, and "It is " or "They are " and ", but" or " and" when bracketed.

#### Scenario: The plate gauntlets
- **WHEN** the card of item 0x153 ("A pair of \x10 plate gauntlets made from a brassy metal. [pq]", quality 223, wear 64) is built
- **THEN** its sentence is "A pair of plate gauntlets made from a brassy metal. They are extremely well made and in good condition."

### Requirement: Short and fixed shipped sentences
A shipped item's description of two bytes or fewer SHALL stay unchanged, and a fixed-name item's SHALL only lose its first 0x10 byte and the byte after it, taking an article when its first byte is at least `Z`.

#### Scenario: A fixed-name pendant
- **WHEN** the card of item 0x58d (seed word −1) is built
- **THEN** its description stays "As you touch the pendant a soothing sensation washes over you."
