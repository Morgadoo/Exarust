# Design

## Approach

Each original power class is a plain Rust struct with a `control` tick
that returns what the game must do (release, link, hide, steer) instead of
reaching into bodies, effects and sounds. The focus goes through the cast
protocol's existing plumbing (`casting::reserve`, `release_reserve`,
`spend_tick`, `start_cast`) so a power and its cast share one pool model;
power ids, motions and sound names come from the cast modules. The rules
are expressed in Exarust's names; arithmetic follows the disassembly's
order and precision (double where the original widens, single where it
narrows first). Reference: `docs/THAUMATURGY.md`.

## Decisions

- **Reserve, not spent.** The base's release takes the held focus back off
  the reserve only, as the original does; the spent focus recovers with
  the pool.
- **Throw holds no reserve.** It spends a per-tick cost (tick ms × cost ×
  0.00125) on every steered tick; the first port read that cost as a
  record word.
- **The vanish phases take how their hold ends.** Vanish holds until it
  is released, spending half its cost; the Long Door has no end event and
  holds 400 ms. Both wait 400 ms, spending their cost, before the body
  vanishes.
- **Quirks kept.** Taking control back writes the caster's status, less
  the control bits, over the released body; a restored Force power's held
  focus is taken down by 0.75 twice with the reserve technique; the
  loyalty target's trust is set to the step rather than summed.

## Open questions

- Which body vector the kinesis and throw steering rotates into their
  −0.2 term (a four-component vector through the body's matrix): answered
  by tracing the sim object's fields with REA when the steering is wired.
- The gamepad stick's length routine (taken to be the vector length):
  REA on the engine's vector routine when the stick is wired.
- Control Mind's binding countdown is started by the Mind cast (another
  lane); the class takes it as given.
