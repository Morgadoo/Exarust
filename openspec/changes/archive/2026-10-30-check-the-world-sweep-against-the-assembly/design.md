# Design

## Context

A DeepSeek sweep lane ported pieces of eleven world components, mostly from
pseudo-code. Review against the instructions kept what matched and replaced
or removed the rest.

## Decisions

- **Main stays authoritative.** Where the lane modelled something Exarust
  already plays (the keyframe track list, the cell light lists, the
  viewport repaint byte), the duplicate is removed instead of reconciled.
- **Blend walk.** The NaN branch follows the jump-on-parity pairs: a NaN
  input splits into two NaN weights. This matches the round-43 branch's
  reading of the same walk, so the two merge to one behaviour.
- **Read-back.** Kept as a small tested model (byte counts, record bits)
  because the record bits are the manager's state; it has no caller, as
  Exarust has no GPU bake.
- **Lever light root.** The arm node's parent stands in for the operative
  node; a root arm with no parent searches from itself (a guess).

## Risks

- The blend walk feeds the running terrain build: weights change only in
  the last bit (remainder order) or for NaN inputs.
- Kind 6 levers with a light under the operative's node now fade instead
  of signalling, as the original does.
