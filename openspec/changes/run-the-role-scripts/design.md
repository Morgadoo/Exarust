# Design

Format page: [`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md#roles-charrolesrdb).

## Reader

`exa_formats::roles` decodes the role database over the generic `.rdb`
framing, and dialogue graphs as a tree (nodes → branch groups → replies)
rather than the original's pointer walks; every walk of the original maps
onto an index path into that tree. Names stay 16-byte arrays because the
lookups compare all 16 bytes (exactly for topics in a running dialogue,
case-blind for activations). Text stays bytes (Windows-1252).

## Interpreter

The original has no tokens: it moves a cursor over the text, and several
of its behaviours only exist because of that (the false branch stepping
onto an LF, the byte after `)`, member names ending only at certain bytes,
the shared variable slot). `exa_script::interp` reproduces the walk
routine by routine with the same cursor arithmetic, reading NUL past the
text. Built-ins go through a `Host` trait so the interpreter has no
knowledge of the world model; the dialogue engine's host adds the running
conversation (`TopicUsed`, `ActivateTopic`, `LastEnc`, `EndDialogue`).
Effects that leave the script layer (game events, inventory, checkpoint
saves) are collected as `Effect`s.

## Engine

`exa_script::dialogue` keeps the original's global state in an `Engine`:
conversations in an arena with reference counts (a live conversation of
the same graph and pair is reused, and may sit twice in the active list,
as in the original), the bubble list, the panel, the pending narration and
the line seed. The characters are a plain `Cast` (roles, relations with
memory blocks, AI contacts) until the AI owns them; the AI's values the
engine reads (attitude, concern, contact state, arousal, out of action)
are fields of it.

## Questions and how they were answered

- Section order inside an entry: the entry-section finder and a corpus
  parse of every entry (all sizes match with 0x1, 0x2, 0x4, 0x8, 0x10,
  0x40; 0x20 alone in the data, placed last as a guess).
- Node and branch layout: the node, group and branch walkers of the
  evaluator, the step and the event routine; the whole corpus parses to
  every size.
- Mood bands: the evaluator's mood routine, read in the disassembly (the
  decompiler mislabels the constants: 70, 30, −30, −60; the −60 band is
  never reached).
- Calm filters: the two node predicates, read from the disassembly
  (Ghidra does not know them as functions).
- Topic memory filing: the open routine (key and entry by relation-ness)
  against the memory load/store and the activation routines.
- Not determined: the run-time field a negative `LastEnc` slot reads, the
  mood code's reader, node flag `0x2000000` and bits 28–31, role flag bit
  8, the type word of role variables.
