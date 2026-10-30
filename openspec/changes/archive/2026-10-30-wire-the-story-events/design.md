# Design

Formats page: [docs/formats/RFC.md](../../../docs/formats/RFC.md) (the
character record's class word and powers chunk).

## Decisions

- **Order.** The live consumer applies the handler's effects in the order
  the handler emits them, after the flow's own (unlock, fragment word). The
  raise-dead grant runs before the handler, only for the raise-dead code on
  the unknown background, because the grant writes the power and the
  original only reaches it there.
- **Player reading.** The memory role comes from the body's roles, Exarust's
  live copy of the record's role list (the record's list is the fallback).
  The voice volume and pitch are the body voice's gains, the fields the
  original scales the groan by.
- **Voice line.** The fragment's call is the body's voice request with line
  2; Exarust's request is the pain voice's path (`PainSounded` with a voice
  and no wound sound), which already holds the line's waits and gains.
- **Narrator.** The three lines are narrator topics; `exa_script::Narrate`
  is the engine's topic request with its delay.
- **Session words.** The background and the fragment word belong to the
  story state; the save writes them in the header byte and the memory-flags
  chunk, as the original does.
- **Random draws.** The process generator the sound bank keeps, as the
  golem's wake uses.

## Open questions

- What the post-process flash draws (its 3 s fade is traced; the pass is
  not ported): REA on the post-process pass.
- The witness event's broadcast to role holders: REA on the role-event unit.
- The experience's kill/pickup hooks: the call sites pass a world object
  whose word has a "not discovered" flag (cleared on the first call) and a
  rank in bits 12 to 14; REA on the two callers to name the objects.
