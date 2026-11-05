# Design

Format pages: none (the faces are ordinary `.rfi` images; the PE resource
tree is the system's format). `docs/UI.md` (Fonts, In-game GUI unit)
describes the behaviour.

## Module images

An image reference carries a source kind. The kind "module resource"
resolves through the system's resource finder on the running module: the
reference's name (inline up to sixteen characters, or a pointer), type
raw data (10). The bytes found are copied and decoded as any `.rfi`. The
interface's five font records reference `Rayform06R_rfi` …
`Rayform16R_rfi` this way; the glyph and symbol sheets reference
`GlphCommon_rfi` and `GUISymbl_rfi`. The installed executable carries
exactly these (plus `ICOMAIN_RFI`) as named raw-data entries, language
2057; names are stored upper case and the finder ignores case.

Exarust parses the installed `Exanima.exe` once (DOS and PE headers, the
optional header's resource directory, the section table to map addresses
to file offsets; type → named entry → first language → data entry) and
keeps the named raw-data payloads. A malformed or missing executable
leaves none (the base-font fallback stays). The lookup after the packs
cannot shadow a pack image: no pack holds these names.

The faces decode as 128 × 256 (06R), 256 × 512 (08R, 10R, 12R) and
512 × 1024 (16R): a 16 × 16 grid whose cell holds the record's glyph
quad, so the existing metrics sample them as the original does.

## Constructor words

Style word / attribute word as each constructor leaves them (the show then
commits: a top-level form with attribute 0x10000 gains style 0x10 and
attributes 0x41; a held one loses 0x20000):

| form | style | attributes |
|---|---|---|
| base form | 0 | 0x30022 |
| journal | 0x3000 | base \| 0x8000 |
| loot window | 0x3000 | base |
| item panel (its own constructor) | 0x80 | base \| 0x100010 |
| item card | 0x3080 | 0x310f0 (written) |
| container popup | card's with 0x80 cleared | 0x830023 (written) |
| trade window | 0x3000 \| 0x800 \| 0x8000 | base \| 0x800000 |
| trade character view | view's \| 0x80 | view's, 0xc000 cleared, 0x90 set |

The hub's record forms keep their traced attribute words; their placement
word is the style word's centring bits.

## Open questions

- The card's tint colour slot (set by the builder for the title) is not
  traced into the form draw; the backdrop stays the palette's.
- The table views' and the board panes' own words (they keep the default
  background).
