# Design

## Card facts

The catalogue keeps one `CardFacts` per record, read once at load. Section
offsets are body-relative (the original's builders read header-relative
words eight bytes earlier). The page order is the builder's: a weapon
section without the withheld flag `0x01000000`, else a body of armour type
1–6, else a shield section without the flag, else the plain page. The tint
byte, rank byte and socket flag are taken from the section of that page,
so a withheld weapon on an apparel record reads the apparel body; the
shield page's rank is its section's `+0x08`.

Names resolve as the original's resolver does over the base tier, whose own
library is the shared `objstrings.rdb`: 0 is none, exactly `0x80000000`
inherits the parent's field (one level, shared library), another id with
bit 31 reads the low 30 bits, any other id reads as it is. The builder
returns no card when the name does not resolve. Without a loaded library
the record's 16-byte name stands in (an Exarust fallback).

## Card icon

The card acquires its icon with the entry flags masked to the low 28 bits,
or with the apparel framing bit when the model flags have `0x1000`, keyed
by the plain key (lit models keep the condition byte and gain the lit bit);
it does not add the streamed or effect bits the panel's object-database
acquire adds, so the card usually holds its own slot and render.

## Icon release and reuse

The manager's release decrements the slot's references; the last one
frees the atlas cell and clears the id (variant and extents stay). The
acquire reuses the first slot without an id before appending, so opening
and closing panels does not grow the cache. The release is queued by the
screen close and drained after the close systems.

## Encumbrance

The live sum is `exa_formats::encumbrance::encumbrance` over the worn
burdens, the shield of the first two hands (the second wins) and the
record's techniques, with ids looked up by their low 30 bits as the spawn
does. It runs on `Changed<Possessions>` except the first frame, writes the
physical and the fighter copies, and so agrees with the spawn value.
