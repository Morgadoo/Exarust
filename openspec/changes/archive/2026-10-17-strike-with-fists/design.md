## Context

How the original turns the left button into a landed punch.

**Request** (the player's behaviour tick, combat mode). The behaviour
compares the mouse buttons with last tick's. A new press (no interaction
pending) is a double click when it comes within 300 ms of the last press's
time, which is then cleared, else it becomes that time; the side is taken
from the cursor's heading against the body's yaw (left by more than 20°,
15° for weapon class 0: the other side); the request is set. While the
button is down the request stands (a held flag), and the side is not
taken again. A release clears the request and its flags. While the
request stands and something is held: the aim and the heading are the
cursor's direction every tick; the thrust flag follows Alt with a weapon
class that has hands bits; and with the energy at least 0.8 × its most
(compared in double precision) the executor runs. The AI calls the same
executor behind the same energy test.

**Executor.** Nothing with no held item. Busy (now before the end of a
released swing's stall): only the released swing's overhead within
300 ms of the last side attack's start, or Feint's renewal — both come
with the swing clock (M22), and Exarust never sets a stall end yet. Free:
the cooldown is 400 ms without a primary striker, else round(300 × the
primary's recoil W17) + 300 ms (single precision, ties to even); it has
passed when the last attack tick plus the cooldown is not after now, and
Break Tangle (absent in Exarust) blocks. Then: a double click's request
starts the overhead unless a stab or an overhead plays; a thrust the stab
unless a stab plays, after the facing gate (the primary weapon's long
axis in the body's frame, forward component f: allowed when f > 0 or
round(−15f) < 6); else, outside the side attacks (motions 5–10), the side
attack of the request's side. The starters set the primary (the right
held item; the left one for the other side with the fists, claws or two
weapons), pick the clip (random without repeat; Attack Mid crouched,
which becomes the side's last pick), and the wrappers clear the side.

**Last attack tick.** The swing routine, run from the body tick's attack
branch on every tick of an attack clip (motions 5–12), stamps the game
clock. The game clock advances 12 ms a simulation tick. So the cooldown
counts from the attack's end, and during an attack it never passes: the
free executor cannot cut a playing attack short.

**Primary striker.** Emptying the hands (equipping the fists) clears it,
so the first unarmed attack waits 400 ms after the last attack tick;
equipping a single weapon makes it the primary at once.

**Strikers.** Each held item's chain ends in a striker body; with the
fists the strikers are the hands themselves. At the strike window's
opening, with weapon class bit 0x80 (fists, claws, two weapons) only the
motion's hand strikes: the overhead the first held item (both with claws,
class 0xC0), side 1 and the stab the first, the combos the second (which
becomes the primary); otherwise every striker. The damage routine measures
a fist's contact along the hand's x axis (negated for the right hand) and
ignores a contact at or past the striker's end (p ≥ 1).

The research notes, with addresses, are git-ignored:
`context/re/master-combat.md` §1–2, §4 (and its "L3 findings" added by
this change), `context/re/combat-stances.md` §7, §13,
`context/re/combat-look-plan-final.md` (L3), `context/re/combat-damage.md`
§1, §2.

The format page this change extends is
[docs/formats/OTHER.md](../../../docs/formats/OTHER.md) (Weapon record).

## Goals / Non-Goals

**Goals:** fists that land; the executor's cooldown and energy gate for
the player and the duellist; hold to repeat; the strike's hand by motion;
`RaceWeapons` from the decoded race stats.

**Non-Goals:** the swing clock, release and stall, the busy executor
(M22); aim nudges (M23); contact detection between bodies and the fist's
own damage path (C21, M30); parries (L10).

## Decisions

- **One executor in `exa_combat::attack`**, pure functions over plain
  inputs (cooldown, energy gate, facing gate, the choice of attack, the
  striking hand, the primary's slot) and an `Attacker` component (last
  attack tick, primary slot) required by every `Fighter`. The player and
  the duellist call it; only the request differs. Times are milliseconds of
  the game clock, `BodyTicks::millis` (12 ms a tick), so the cooldowns stop
  with a paused or slowed simulation as the original's.
- **The request runs in the behaviour tick** (`FixedPreUpdate`, before the
  steering), reading the button state each tick against last tick's, as
  the original does. A latch before the fixed loop keeps a press that came
  and went in a frame that ran no tick (a scripted click, a slow run). The
  frame's cursor facing stays per frame (it reads the request).
- **The last attack tick is stamped in the fixed step before the
  character ticks** (the original's attack branch runs before the clip
  clock advances), for every fighter with an attack clip playing.
- **Fists are `Blade`s**: a child of each hand part turned so that its
  local +Y runs from the hand's origin toward its box along x (the side the
  box centre lies on), as long as the record's W21. `Blade` gains the held
  slot it fills. Parts keep their RFC box half extents (`PartBounds`,
  `ModelNode`) so that dressing measures the hands without parsing the body
  model again.
- **Which strikers strike** is decided per attack (`Attacking` keeps the
  motion and the fighter's weapon class): with class bit 0x80 the motion's
  hand, else armed blades only. That also keeps the fists out of a
  torch-holder's attacks (class 0x10).
- **Knuckles.** Exarust's segment ends on the hand box's far face, so a
  straight punch meets a part exactly there and the original's p ≥ 1 rule
  would drop it; the contact of a fist is kept 0.1 cm inside its far end.
  Unarmed blows do not use p otherwise (their zone is 1 everywhere until
  M30), so only the drop is avoided.
- **Double click during an attack.** L1 let the overhead cut a side attack
  short on the second click. The original's free executor cannot (the
  cooldown never passes during an attack); its immediate overhead needs a
  released, stalling swing (M22). Without the stall, Exarust follows the
  free executor: a held second press gives the overhead once the attack
  ended and its cooldown passed.
- **Energy.** The body's daze energy (`Drive::energy`) against the human's
  most, `MAX_ENERGY` (1; the feral's 1 − 0.1·feral comes with M41). A body
  without muscles counts as rested.
- **Race weaponry.** `RaceWeapons::of_stats(&Race::stats)` (the unarmed
  reach converted from metres to the record's cm); a `RaceWeaponry`
  component (race index and fields) on placed characters, the human's by
  default; held weapons and fists are held by it.

## Risks / Trade-offs

- The duellist's strikes keep their 2.2 s interval and 1.2 m range (Exarust
  stand-ins until L11); the executor only adds the energy gate and the
  cooldown, which its interval already exceeds.
- The duellist's harmless hilt contacts still stop the player's attack (the
  stand-in reaction until M22); in the close duel some punches end early.

## Open Questions

- Retail request RR32 (unarmed punches ahead, left, double click, right
  after a hit) would show in the live game how a quick double click
  behaves once the release stalls the first punch (the busy executor's
  overhead, M22), and the 330 ms spacing. The clock and the comparisons
  were read in the exe.
