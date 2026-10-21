# Design

## Context

The arena's player-data saver writes its chunks through one stream: a word
is four little-endian bytes, a byte run is copied as it is, a string is a
byte count and the bytes, and a chunk is its id, a length patched when it
ends (the bytes after the length word), and the payload. Rule page:
[`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md) (Saves).

## Goals / Non-Goals

**Goals:** the three payloads' fields in order, each readable and writable,
an early end an error.

**Non-Goals:** the hire list, employees and merchant chunks; writing a
retail save.

## Decisions

1. The words the readers skip (the company's unused word, each special's
   fourth word) are dropped on reading and written as 0.
2. A negative count reads no entries, as the loaders' loops do.
3. Unnamed words keep neutral names (`second`, `words`, `log_word`,
   `third`, `fourth`) and say what is not traced.

## Open questions

- What the company's second string and three words hold: answered by
  reading their writers in the company creation form (REA).
- Whether these chunks parse from a real career save: answered by a save
  written by the original (a capture), checked with a corpus test that
  skips without one.
