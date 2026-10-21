# Design

## Approach

The rules are plain structs and functions: a cast takes the two bodies and
the scene and returns what the world must do (sounds, the overlay, the
spirit screen, a revival's refreshes); a force effect's tick takes the
view (aim point, hand, facing) and the caster's cast body and returns its
events. Arithmetic follows the disassembly's order and precision; the
engine's helpers (`thaum::engine`, `nav_camera::planar_yaw`,
`exa_ai::combatant::asin_fast`) are reused, not re-derived. Reference:
`docs/THAUMATURGY.md`.

## Decisions

- **One pool model.** The Mind casts spend through `casting::spend` and
  `casting::reserve` and create the `thaum::power::mind` classes, so the
  casts and the power classes share their focus and state.
- **One power list.** A Mind body's power list holds every power and
  effect on it, each with the class the original creates; other domains'
  entries are kept as base powers for the slot count.
- **Quirks kept.** Calm, Trust and Loyalty walk every contact; the
  revival stores its decay before testing it; the blast's separation
  divides by a zero distance; the torrent's removal copies the record
  before the last; the character stun's 0.008 above 0.08.
- **Not ported, said so.** The ribbon vertex builds, the mesh contact
  tests and the GL passes stay out; the contact arithmetic takes their
  results as inputs.

## Open questions

- The player behaviour's turn spring (its rate constants): REA on the
  behaviour's turning routine when the torrent is wired to the player.
- The body fields the revival's stamina reads (load, build): named by
  their use; a capture of a raised corpse would confirm them.
- The AI fields the mood weights read (anger, dread, arousal): named by
  their use in the colours; the AI lane's contact notes will confirm them.
