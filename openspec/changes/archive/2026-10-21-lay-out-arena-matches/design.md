# Design

## Context

Each match template carries a setup routine. They share three placements:
a fixed opponent template (identity basis, 5 cm up and 2 m ahead of the
spawn spot), a fixed company template (turned half a turn, 2 m behind) and
a ring routine that turns a fraction of a turn into a direction with the
engine's sine and cosine and stands the fighter 3.5 m against it. The
fray computes its six angles itself. Rule page:
[`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md) (Arena tables).

## Goals / Non-Goals

**Goals:** the eleven layouts as data a caller spawns from, with the
original's float order; the next round's choice; play mode staging them.

**Non-Goals:** generating bodies and kits, the beast, company entrants
beyond the player, inter-opponent AI.

## Decisions

1. Places are centimetres in the arena's frame with a forward direction
   (the basis's z row); play mode converts to metres and a yaw
   `atan2(x, z)`.
2. The ring uses `thaum::engine::sin_cos`, the same engine routine.
3. The one-on-one layout equals `exa_world::start`'s duel ring (2 m either
   side), so `--opponent` alone is unchanged.
4. `--arena-match` implies `--opponent`; an entrant that is not the player
   and the beast are logged and not spawned.

## Open questions

- The look matrix's x and y rows (only z is used here): read with REA when
  a spawned fighter needs its full frame.

## Risks

- Opponents of a fray all fight the player: Exarust's duellist AI has no
  teams. The tick still counts them as one side.
