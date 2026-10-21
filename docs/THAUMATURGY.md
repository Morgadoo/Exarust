# Thaumaturgy

The original's powers as Exarust runs them: how a cast is let through and
priced, what each Force, Energy and Displacement power makes, and the
effects (electric arcs, the force barricade, displacement wisps) that run as
simulations in the game. The Mind domain, the power tree and the
spirit form are in `exa_game::thaum` and `exa_game::mind`. Lengths are in
cm, as the original's effects keep them; the effects run at the 83 Hz body
tick.

Evidence: the decompiled cast tables, front ends, gates, casts, creators,
AI evaluators and effect classes of `Exanima.exe`; research notes in
`context/re/thaum-casts.md`, `thaum-arcs.md`, `thaum-barricade.md` and
`thaum-wisps.md`.

## Casting

`exa_game::thaum::casting`.

- **What a cast returns**: 1 started, 0 released (the power was charging:
  pressing it again lets it go), 2 ended (a persistent power already ran
  and is told to end), −1 refused (short of focus, no free slot, no
  weapon), −2 failed and −3 "not now" from the state gate.
- **The state gate** (Force and Energy): an attacking body waits ("not
  now") while its swing is in one of its 0x34 states and fails otherwise; a
  moving body fails unless it is stopping, then waits at a pace of 2 or
  more; a casting body asks for the release and waits; a collapsing body
  fails; a standing body passes unless it is switching weapons; a body on
  the ground fails, and one getting up waits past 0.9 of its clip.
- **The Displacement gates** test the swing the same way, fail a vanished
  body (the instant gate) and, at a clip rate of 1 or more, a collapsing
  (instant) or collapsing or dead (held) body; otherwise they pass on the
  caller's own word, the id of the power being cast.
- **Slots**: the highest chosen thaumaturgy technique of 2 to 4 among the
  record's tiers (one tier for each 5000 of its thaumaturgy word), one less
  without the focus item, less one for each running persistent power.
- **Prices**: the row is found by the id under `0x0e0f_ffff`; an instant
  power cast again with the repeat technique costs half; a sustained power
  costs the tick in milliseconds times its cost times 0.001 (Force and
  Displacement) or over 1000 (Energy), half again with the sustain
  technique; persistent powers hold their cost (three quarters with the
  reserve technique) until they end.
- **Focus factors**: blast, bolt and shock need 1.2 times the cost, the
  long door 0.8 times (both in double precision, so a focus of exactly
  0.12 is short of a 0.1 blast), barrier twice and vanish 80 times.

## Force

`exa_game::thaum::force`. The table: blast 0x201 (0.10), bolt 0x202
(0.10), torrent 0x10203 (0.10 a second), wave 0x204 (0.20), barrier 0x10205
(0.07 a second), barricade 0x20206 and bubble 0x20207 (0.25), shell 0x20208
(0.35), kinesis 0x10209 (0.02 a second), throw 0x20A (0.10), strikes
0x2020B (0.30). Kinesis releases while the controls drag something; throw
takes what a charging kinesis holds, or a loose body (a character only
dead, a prop of mobility 0.01 or more) under the cursor; strikes need a
weapon in hand and cast motion 0x25. The barricade stands at the cursor's
ground point, 100 cm up from the floor 210 cm below, facing away from the
caster; the barrier and the bubble are domes 100 cm above the caster,
radius 100 and 120. The AI casts blast (focus, facing within 1, 6 m),
barrier (facing 1 or more, 3 m), shell and strikes (cost within half the
focus, facing within 0.5, a free slot).

## Energy

`exa_game::thaum::energy`. Shock 0x401 (0.10) starts cast motion 0x27;
its charge lasts the cast clip's length over 1200 ms and costs the cost
times that times 1.2; its hand arcs ride the right hand. Shock weapons
0x2040B (0.30) put arcs on up to four held weapons and follow weapon
changes; a struck body's character takes a 0.01 shock and a spark burst.
The AI shocks within 5 m.

## Displacement

`exa_game::thaum::displacement`. Vanish 0x10601 (0.10 a second), the long
door 0x602 (0.20, to the cursor's point), constant displacement 0x20603
(0.35, persistent), the summoning 0x604 (0.35, cast motion 0x28, spent at
once). A constantly displaced body lets a blow through when a percentage
roll is at most `100 >> (s + 1)` (s ≥ 0) or `100 − (100 >> (1 − s))`
(s < 0) for its streak s: 50 % at first, 25 % after a pass, 75 % after a
miss. A passing weapon loses its collisions for 500 ms and leaves a wisp
at the contact. The AI's long door and summoning words carry their cost.

## Electric arcs

`exa_game::thaum::arcs`. Tendrils of points grow from a source a segment
(4·size) at a time, open branches where they bend and lose points where
they fold, trade tails with branches that outlive them, jitter (one tick
in 30 they jump), keep 0.9 of their motion and stay within half a segment
of their neighbours' middle, end at the level, and shock the first
character they touch (once per character with the shock-once flag),
reaching a branch toward it. Each frame the light flickers within 0.75–1.2,
the hum's gain and pitch follow the share of points in use, and a
camera-facing ribbon runs through Hermite midpoints, its ends transparent,
its points feeding up to 16 light points merged within 40 cm.

## The barricade

`exa_game::thaum::barricade`. A 600 × 200 cm wall, 5 cm thick with rounded
edges, trimmed 40 cm past the level on each side, grows from its middle
over 1.6 s, stamps its line into the navigation grid, ripples where it is
struck (at most 8 ripples), shoves body spheres along its normal (more
from the front), knocks a character in front back, throws it off balance,
interrupts an attack against it, and falls over 750 ms when its power ends
(also once the caster is 20 m away).

## Displacement wisps

`exa_game::thaum::wisps`. Rings of eight nodes per strand, 40 cm apart,
rising from anchors on a phased body (per bone) or from a blow's contact;
each node darkens with age, grows 0.1 a tick and drifts on; the strands are
drawn as smoothed camera-facing strips.

## In the game

`exa_game::thaum::live::ThaumEffectsPlugin` runs live arcs, barricades and
wisps after the bodies: arcs follow their hand, end at the static level and
shock other fighters: a shock lands once the game clock is within 150 ms
of the fighter's next shock time, which then moves 500 ms plus a draw below
200 ms of the engine's generator ahead, and a fighter that is not
collapsing or dead loses the strength in yellow health (the rest of the
shock reaction, its motion and stagger, is not wired); a barricade pushes
the particles of every body part it holds (`Ragdoll::push_sphere`), wakes
the body for 32 ticks, knocks a character back a step and sets its
instability, and each part it strikes in front lowers the character's
energy again while it is above 0.25;
wisps ride their parts. The player cannot cast yet (there is no power bar):
`--debug-arcs`, `--debug-barricade` and `--debug-wisps` put the effects on
the player ([DEBUGGING.md](DEBUGGING.md)).

### Drawing

The effects' meshes come from their own builders but are drawn with
`exa_render::glow::GlowMaterial`, an additive stand-in: the original's
electricity program, `effectforce` and `effectfbrcd` (with the animated
force noise) are not ported, nor are the lights, sounds, the navigation
stamp or the AI's sight of a barricade.
