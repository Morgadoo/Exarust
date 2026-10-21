# Design

## Decisions

- The id index keeps entries as (id, offset) pairs in id order with the
  payload after them; a block's first word is its total length including
  itself. Removing a block closes the hole and lowers every later offset;
  deleting stores an empty block and removes the entry.
- Locale names are compared as 16 raw bytes against the lower-cased stored
  name, so a lookup never allocates.
- The memory archive keeps file order and one payload blob. Its writer is
  Exarust's own addition (the original only serialises through its pack
  layer) and is labelled as such.
- The command registry matches the first command whose name is a prefix of
  the line followed by a space or the end of the line, handing the rest as
  arguments. The behaviours Exarust cannot confirm are labelled guesses.
- Library code that nothing calls yet stays unwired on purpose: wiring it
  would add startup cost for no visible effect.

## Open reverse-engineering questions

- What `msm` does in the original beyond its handler name: answered later
  with REA on the mesh-subdivision routine; today it only logs.
- Whether the stream thread's roles match the guess in the docs: needs a
  capture of the running game.

## Formats

Extends `docs/formats/OTHER.md` and `docs/formats/RPK.md`.
