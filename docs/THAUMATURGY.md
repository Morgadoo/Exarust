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

## Power classes

`exa_game::thaum::power`. The casts hand their work to these classes; each
has a state (1 ready, 0 running, 2 ending), the focus it holds and,
where it names one, its target.

- **The base** (`PowerBase`): taking focus adds it to what the body has
  spent, three quarters with the reserve technique; releasing it takes the
  same amount back. A control event 2 marks a power ending and answers 0;
  the next tick releases its focus and answers 2. A persistent power (id
  bit 0x2_0000) publishes HUD word 2 while it runs and 12 with the clock
  and a 200 ms fade once it ends. The power list is walked by id (an index,
  and an order to end one), a saved `(id, size)` list rebuilds Force
  powers (three quarters of their cost with the reserve technique), and AI
  evaluation dispatches on the id's domain byte (0x200 Force, 0x400
  Energy, 0x600 Displacement; anything else answers 0x400_0000).
- **Mind**: Sense Minds (continuous, overlay on while it runs; instant,
  end stamp plus a 2 s fade) and Mask Mind (continuous, body bit 0x1_0000;
  instant, 15 s with an intensity that fades between 5 and 10 s over a
  10 s square curve and a second that fades between 10 and 15 s, at 0.9
  and 0.8). Confusion and Enrage clear body bits 0x4_0000 and 0x8_0000
  after 15 s; Trust and Suspicion last 20 s. Loyalty binds a living target
  (its marker id 0x10010A), builds the target's trust toward 10 by a ten-
  thousandth of the gap a tick, and after 800 ticks pulls its strength
  toward `((relation + 100)·0.005)²·100·0.05` while keeping 0.95;
  ending one half ends the other. Command Dead (0x2010D) does the same
  with marker 0x10010D. Control Mind (0x2010C) spends 0.002 of focus a
  tick for 2 s, then hands the player to the target body (handover bit
  0x10_0000 on the caster, 0x60_0000 on the target) with a possession
  marker (0x10010C) that ends the caster's half when it ends. Spirit Form
  drains its tick cost, clamps the focus at zero and fades over 1.2 s
  through `exa_game::mind`.
- **Force**: Shell halves a blow's energy (the body's hit damping is 0.5) and clears
  it at the end; a bubble or barricade ends once its centre is more than
  20 m from the caster; Strikes sets the strike factor 1.0 and the swing
  factor 3.0, keeps at most four weapon auras and reports a swing of an
  armed weapon; Kinesis steers its held body by its reach (half its
  radius plus one) toward the aim (`1/dist`, 0.02 within 50 cm), turns it
  by `reach²·0.001` a tick and stamps its user for 2 s; Throw lifts at
  `record·strength·0.00125`, launches along the aim at
  `mass/(mass + 0.1)` (a fiftieth within 50 cm), drops the shell after
  800 ms and restores the materials 200 ms later.
- **Energy**: Shock charges by its rate, fires one arc burst when it
  passes 1, spends its focus while charging and answers 2 on the next
  cycle; Shock Weapons starts motion 0x25 and clears its weapon arcs when
  it ends.
- **Displacement**: the vanish effect hides the body (status bit 0x800),
  fades out over 300 ms, holds (Vanish spends half its cost a tick), moves
  the body on release (the long door to the grid's place, the summon to
  the demon's), fades in over 400 ms and cleans up after 300 ms. Constant
  Displacement's shell grows 0.002 a tick up to 0.12. The summon waits
  2 s, fades over 3 s, waits 1 s and brings in `demon a01` with role 0x21
  and a 300 relation both ways.

The classes are plain functions with tests; nothing in the running game
calls them yet, and the combat factors they publish are not read by
`exa_combat`'s hit formula. The player swap, the spirit body's character
generation and the effect meshes are descriptions here, not spawned
entities.

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
