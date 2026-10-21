# Design

## Context

The original links `steam_api64` at run time and reads its controller only
through Steam Input action sets. Exarust must not link Steam, so the sets'
rules are re-expressed over Bevy's `Gamepad` (gilrs), with a default table
standing in for the Steam manifest's bindings, which are not in the
executable. The rules live in `exa_game::gamepad` as plain readers so they
are tested without Bevy; a small plugin samples the first connected pad
each frame. Docs: [PLAYER.md](../../../docs/PLAYER.md) (Gamepad); no
`docs/formats/` page changes (no file format is read).

## Goals / Non-Goals

**Goals:**

- Every rule the two Steam Input components apply to the stick and button
  data, at the listing's precision (single or double compares, rounding,
  operand order, NaN branches).
- The pad drives the player in controller mode; keyboard and mouse play is
  unchanged.

**Non-Goals:**

- Steam API glue, a drawn GUI form toolkit, glyph prompts.

## Decisions

- **Bevy's gamepad, identity axis settings.** The pad's sticks and triggers
  are given an identity response so the original's own dead zones see the
  raw values.
- **The look rate is the camera's.** The look routine multiplies by the
  navigation unit's per-pixel rate (4.8 rad over the screen width) and
  the camera's sensitivity; the chase camera passes `orbit_per_pixel` of
  its window and sensitivity 1. AimSpeed is not used here: the listing's
  only reader of it is the player behaviour's aim cursor ease.
- **Stepped zoom on the chase camera.** Exarust's chase camera zoom is in
  the original's units (800–2000, stops of 240), so the step routine's
  arithmetic applies to it directly.
- **Steering by keys.** The original hands the stick's world vector to its
  player behaviour as a wanted direction; Exarust's exploration steering is
  key-driven, so the gamepad layer translates the stick into the keys a
  player would hold. This is Exarust's own and is labelled so.
- **Controller mode.** The pad drives the player while a pad is connected
  and the controller flag is on (the Controls page's switch, or a pad used
  this session). The original runs its gameplay reader whenever Steam
  reports a controller; Exarust's flag keeps a stray pad from moving the
  player.

## Open questions

- The step modes' and raw-aim mode's writers, the command word's consumer
  and the exit hooks are outside these components; answered by REA on the
  player controls' hotkey handler (power bar and dialogue flags) and left
  as labelled unknowns otherwise.
- The manifest's physical bindings cannot be recovered from the
  executable; the default table is Exarust's and documented as such.
- The basis normalisation uses the processor's approximate reciprocal
  square root; its bit pattern differs by vendor, so Exarust normalises
  exactly.
