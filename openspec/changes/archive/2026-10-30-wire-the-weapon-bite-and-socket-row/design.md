# Design

## Bite

The original keeps the bite as a link record on the weapon body. Exarust's
held item is its own `Ragdoll`, so `BiteHold` is a cross-body solver stage
like the grip stage: each bite names the weapon part and the struck part by
entity; the stage finds their bodies among the awake ones each pass and
runs the original's grab law (`pull_grab_across`). On a break it drops
every bite of that weapon part on that part, lowers the struck part's and
its neighbours' grab bit and the weapon's bit, and records the break for
the log.

The hit reaction runs in `PostUpdate`; it writes `BiteTaken`, and
`open_bites` (chained after it) builds the link: the unit is the
attacker's root link-iteration count, the weapon part the one nearest the
contact, the points the contact's weights on both parts as they stand,
and the axis the blade velocity converted to cm a tick in the weapon
part's settled frame. The next tick's solver pulls it.

Open questions and how they were answered:
- What `n` is: the root's link-iteration word (REA: its writers in the
  character constructor, the body tick's ramp and death; the global
  default 2).
- Rounding of the count: the conversion instruction (to nearest).
- The gate: the routine's double compare with 0.1 and the class test.
- The layers: the layer routine writes the layer mask word.

Exarust stand-in: Exarust builds a held item body only for models with
rigid nodes; the debug duellist's sword and the factory spear have none,
so the bite then holds the part that carries the weapon (the holder's
hand), found among the striker's ancestors. Evidence: the GPU runs logged
"no body" for the weapon before this fallback.

Guess kept: Exarust's settled frame is the Gram–Schmidt rotation, not the
original's affine matrix (as in tetra7).

## Socket row

The card's rows are Exarust's layout, drawn at text baselines. The socket
row sits after the striking rows, a unit under the last row's baseline, at
the rows' left, 0x18 × unit/4 high, its cells at the
level's large-icon side (`gui_level(level).large_icon`). The cells are
resampled from the two sheets into one atlas once per side. The crystals'
white alpha takes the card's fade like the card icon [guess].

Formats page: none (no data format changes); `docs/UI.md` covers the card.
