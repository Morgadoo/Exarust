# Packs (`.rpk`) and the project file (`.rfp`)

Status: **verified** — `exa_rpk` opens all 12 retail packs and every nested
pack inside `Resource.rpk`; entry bounds check out against file sizes.

## Pack layout

```
u32  magic        0xAFBF0C01   (01 0C BF AF)
u32  table_len    bytes of the table, a multiple of 32
     table        table_len / 32 records:
       char  name[16]   NUL-padded; a 16-character name has no terminator
       u32   offset     relative to the end of the table (8 + table_len)
       u32   size
       u8    reserved[8]  zero in every shipped pack
     data         entry payloads, packed in table order
```

- Names are lowercase in the table; references from models/materials use
  mixed case (`GrassPalA01D`), so lookups are case-insensitive.
- Main packs store bare names (`armet___aad`). `Resource.rpk` keeps
  extensions (`arena7.rfc`, `humandiff.rfi`) and nests further packs.
- No compression or encryption at this level. Payloads are contiguous, so a
  pack can be memory-mapped and served zero-copy (Textures.rpk is 3.8 GB).
- Sorted by name (ASCII), which allows binary search, but the reader uses a
  hash map.

### Variant `02 0C BF AF`

`Resource/objstrings.rdb` starts with magic `0xAFBF0C02` and a different
record shape (16-byte records). Not parsed yet.

## Project file (`.rfp`)

`Exanima.rfp` (153 bytes) names the packs the engine mounts:

```
u32  magic   0xAFDFBD10   (10 BD DF AF)
u32  0
char name[20]             "Exanima"
str  resource_pack        "Resource"
u32  n; str[n] object libraries     ["Objlib"]
u32  n; str[n] texture libraries    ["Textures"]
u32  n; str[n] material libraries   ["Resource\Common.rml", "Resource\Underworld0.rml", "Resource\UnderworldB.rml"]
str = u32 length + bytes (no terminator)
```

The interpretation of the three counts is inferred (it consumes the file
exactly). The other packs (`Apparel`, `Characters`, `CharExt`, `Components`,
`ComponentTex`, `Emblems`, `Factories`, `Objects`, `Sound`) are not listed and
are mounted by the engine by name.

## Inventory of a retail install

| pack | entries | size | content (by magic) |
|---|---:|---:|---|
| Resource | 243 | 438 MiB | 105 chunk trees (levels, base meshes, level pieces), 86 images, 17 terrains, 15 terrain tables, 9 nested packs, 5 databases, 4 powers, 1 skeleton |
| Objlib | 4926 | 213 MiB | static props and level pieces (chunk trees) |
| Textures | 3320 | 3.6 GiB | world textures (3318 rfi, 2 DDS) |
| Objects | 1253 | 35 MiB | interactive items (chunk trees) |
| Characters | 356 | 34 MiB | wearable meshes |
| CharExt | 12 | 0.3 MiB | hair |
| Components | 1001 | 6.8 MiB | weapon parts |
| ComponentTex | 3361 | 120 MiB | weapon part textures |
| Apparel | 1939 | 1.1 GiB | apparel textures |
| Emblems | 135 | 10.5 MiB | heraldry |
| Factories | 106 | 0.2 MiB | weapon recipes |
| Sound | 1131 | 2.3 GiB | PCM WAVE |

Reproduce with `cargo exa info` and `cargo exa stats <pack>`.
