# Design

## Behaviour

A package is `u32 magic`, `u32 count` (including the sentinel), then `count` 24-byte entries `{u32 offset, u32 size, u8 name[16]}`. Entry 0 is a sentinel: offset 0, size 0x10, empty name. Entry 1 starts at 0x2000; each later entry starts at `round_up(previous offset + previous size, 0x2000)`. Section names are exactly 16 bytes, NUL-padded and case-sensitive. Sections are unpadded inside their slot.

Chunks are `u32 id`, `u32 len`, `payload[len]`. Player headers are 48 bytes; the story version is `0x6F5AEA14`, the arena `0xE8AAEA10`, followed by the 16-byte level, 12-byte clock and four dwords.

## Model

`exa_formats::save_package` exposes the container, chunks and headers.

## Open questions

The scene exporters, DB blocks, checkpoints and file naming.

## Docs

`XA_SaveGames` is **12%**. The headline rises.
