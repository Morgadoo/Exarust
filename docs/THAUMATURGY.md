# Thaumaturgy

The original's powers as Exarust runs them: how a cast is let through and
priced, what each Force, Energy, Displacement and Mind power makes, and the
effects (electric arcs, the force barricade, displacement wisps, the force
blast and torrent) that run as simulations in the game. The Mind domain's
rules are in `exa_game::mind`; the force blast and torrent rules and their
draw geometry and lights are in `exa_game::effects::blast` and
`exa_game::effects::torrent`. Lengths are in cm, as the original's effects
keep them; the effects run at the 83 Hz body tick.

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
where it names one, its target. They use the cast protocol's focus
plumbing (`casting::reserve`, `release_reserve`, `spend_tick`,
`start_cast`) and the body's power list.

- **The base** (`PowerBase`): taking focus adds it to the body's reserve
  and spent focus, three quarters with the reserve technique, and to what
  the power holds; releasing it takes the held amount back off the
  reserve only. A control event 2 marks a power ending and answers 0; the
  next tick releases its focus and answers 2. A persistent power (id bit
  0x2_0000) publishes HUD word 2 while it runs and 12 with the clock and a
  200 ms fade once it ends. The record's known powers are searched by id,
  a running power is ended by id, a saved `(id, size)` list rebuilds Force
  powers (the rebuild reserves three quarters with the technique, and the
  load then takes the held focus down by 0.75 once more), and AI
  evaluation dispatches on the id's domain byte (0x200 Force, 0x400
  Energy, 0x600 Displacement; anything else answers 0x400_0000). Power
  Glean teaches Vanish or else Shock from a shaper dying within 20 m (a
  distance that is not a number passes) to a player who knows at least
  one power and none of that domain.
- **Mind**: Sense Minds (continuous, overlay on while it runs; instant,
  end stamp plus a 2 s fade added in 64 bits) and Mask Mind (continuous,
  status bit 0x10000; instant, 15 s with an intensity that fades between
  5 and 15 s over a 10 s square curve and a second that fades between 10
  and 15 s, at 0.9 and 0.8, and a HUD row that turns timed for the last
  5 s, ending on the real clock). Confusion and Enrage clear status bits
  0x4_0000 and 0x8_0000 once the unsigned 32-bit elapsed time passes 15 s
  (the state word is not written); Trust and Suspicion last 20 s. Loyalty
  binds a target that is not collapsing (marker 0x10010A); the target's
  trust is set to a ten-thousandth of its gap to 10 each tick while below
  10, and after 800 ticks its strength blends `((relation + 100)·0.005)²·100`
  (double until narrowed) at 0.05 with 0.95 of the old; its limit test
  adds 10 when the loyal body has technique bit 0x10000; ending one half
  ends the other, and the target's end runs the experience hook and plays
  `ThmMnd_E01` at 0.15. Command Dead (0x2010D) does the same with marker
  0x10010D. Control Mind (0x2010C) counts its binding down by the double
  0.002 and spends the single 0.002 a tick; after 2 s it hands the player
  to the target body (handover bit 0x10_0000 on the caster, 0x60_0000 on
  the target) with a possession marker (0x10010C) that ends the caster's
  half when it ends; taking control back writes the caster's status less
  the control bits over the released body. Spirit Form drains its tick
  cost, clamps the focus at zero below the double 0.001 and fades over
  1.2 s through `exa_game::mind`; the spirit body is a race 0xE00 record
  sharing the caster's known powers, 60 cm along its facing, with its
  focus pool, techniques and running powers.
- **Force**: Shell halves a blow's energy (the body's hit damping is 0.5)
  and clears it at the end; a bubble or barricade ends once its centre is
  more than 20 m from the caster; Strikes sets the strike factor 1.0 and
  the swing factor 3.0, keeps at most four weapon auras, ripples an aura
  at half a swing's strength and plays `thmfrc_brf` as it ends; Kinesis
  steers its held body at its reach (half its radius plus one) along the
  unit direction to the aim (the direction times 0.02 within 50 cm or for
  a distance that is not a number), turns it by `reach²` times the double
  0.001 a tick by the stick or the key pairs (each pair if/else), holds a
  pick at a third of its extents' sum and stamps its user for 2 s; Throw
  holds no reserve: its lift aims 100 cm above the body and spends
  `tick·cost·0.00125` every steered tick at `mass/(0.1 + mass)` of the
  unit direction, drops the shell after 800 ms and coasts at six times
  that share toward the caster's aim point (ending at once within 100 cm),
  and restores the materials 200 ms later.
- **Energy**: Shock's first tick installs its cast hook and plays the
  charge; it charges by its rate, spends its focus while charging, fires
  one arc burst when it passes 1 (clearing the cast hook and id) and
  answers 2 on the next cycle; Shock Weapons starts motion 0x25 and clears
  its weapon arcs when it ends.
- **Displacement**: the vanish effect waits 400 ms spending its tick cost,
  hides the body (status bit 0x800), fades it out over 300 ms, holds
  (Vanish until released, spending half its cost a tick; the Long Door for
  400 ms, spending nothing), moves the body, fades it in over 400 ms (the
  share narrowed before it is taken from 1) and finishes after 300 ms; a
  shell shared with Constant Displacement goes back to it. Constant
  Displacement's hum rises by the double 0.002 each tick while under 0.12.
  The summon brings in `demon a01` with role 0x21 and a 300 relation both
  ways, waits 2 s, fades the caster over 3 s and finishes 1 s later.

The classes are plain functions with tests; nothing in the running game
calls them yet (the player has no power bar), and the combat factors they
publish are not read by `exa_combat`'s hit formula. The player swap, the
spirit body's character generation, the kinesis aim sweep and the effect
meshes are descriptions here, not spawned entities.

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

## Mind

`exa_game::mind`. The table has sixteen rows: sense 0x101 (0.13) and its
persistent form 0x20101 (0.27), mask 0x102 (0.12) and 0x20102 (0.24),
spirit form 0x10106 (0.025), calm 0x103 (0.14), enrage 0x104 (0.16),
confusion 0x105 (0.17), the loyalty release 0x109 (0.19) and bind 0x2010A
(0.29), suspicion 0x10B (0.19), possession 0x2010C (0.35), command over the
dead 0x2010D (0.25), fear 0x107 (0.18), embolden 0x108 (0.18) and raise dead
0x10F (0.30).

- **Gates**: most casts need focus of 0.3 times the cost, compared in double
  precision with an unordered compare that lets a NaN through. Record types
  1 to 3 are living and 4 and 5 undead; a cast on an undead body spends the
  cost and does nothing. Suspicion and possession need the whole cost;
  possession and raise dead also refuse the undead bit 0x10 on the target's
  status word and the character bit 0x40.
- **Calm**: a concern with a decay of 10 or less becomes four times itself
  with its decay cleared; a larger one loses 10 and gains 30. Aggression
  loses 10, the fear value 50 and the courage 1, each floored at zero (a
  NaN is left), and the calm factor is set to 1.
- **Enrage** adds 50 to the target's aggression and its 0x104 effect;
  **confusion** zeroes the concerns and adds 50 of aggression with the 0x105
  effect; **fear** adds 20 of the fear value up to 40; **embolden** adds 50
  of courage up to 80; **suspicion** makes the 0x10010B effect. The effects'
  sounds are `ThmMnd_R01`, `C02`, `T01`, `S01`, `F01`, `C03`, `C01` and
  `A01` at volumes 0.1 to 0.3.
- **Loyalty**: a bind needs a living target, no other bind of the caster's
  and a free power; it makes the 0x10010A effect with a relation level
  `(((r + 100)·0.005)²·100)`, sets the relation to 30 and gives the target
  the 0x2010A power. A second cast on the bound target ends the bind. The
  release (0x100109) halves the relation weights and sets the relation to
  30 again.
- **Sense and mask** are cast on the caster. Sense runs 4000 ms, or the
  focus's share of it; a persistent sense ends a running one. Mask needs a
  free mask slot and sets body flag 0x10000; its persistent form ends a
  running one. The overlay fades over 2 s, the sight nodes are picked every
  4 ms, and a grace window of 5000 ms (a corpse) or 3000 ms (another tracked
  body) keeps a node alive.
- **Possession** needs a living body without the resistance bit 0x10000 and
  a free power; it makes the 0x2010C effect on the caster (the success
  spends nothing; a resist or an undead target spends half and plays
  `ThmMnd_E02`). **Spirit form** needs focus above 0.05, makes the 0x10106
  effect and the 0x1000000 body flag, and ends on a second cast.
- **Raise dead** and **command over the dead** need a corpse (body flags
  0xA00). The revival sets the raised status bit, the condition word 8, the
  speed `1 − 0.1·strength`, the slow tick
  `round((1 − 0.25·strength)·1660/83)` (19 at strength 0.2) and the strength
  scale `1.25 − 0.25·strength²`. Raise dead binds nothing; command makes the
  0x10010D effect on the corpse and the 0x2010D power on the caster.
- **Draw descriptions**: the sight screen is a 256² texture with a
  16-segment, 40 cm sphere and four noise octaves at 0.5; the spirit screen
  is 512² at 0.8. Each tracked body's draw carries its billboard axes, its
  three colours and thresholds and its texture offset; the unit's shutdown
  is empty and its two shader source strings are released.
- **Target picking** takes a character of body class 1 without effect flag
  0x10, alive unless the dead powers ask, seen by the view unless Sense
  Minds runs, and within `n·scale·0.5·40/depth` of the screen centre. The
  nearest ratio below 1 wins.
- **Prices and restore**: an instant power repeated with the repeat
  technique costs half; a sustained one costs the tick times its table cost
  over 1000, half again with the sustain technique; a persistent one keeps
  its cost. A restored persistent sense or mask holds three quarters with
  the reserve technique.

## Force blast and torrent

`exa_game::effects::blast` and `exa_game::effects::torrent`.

- **Blast** (0x201, and the bolt and wave through their tuning): sixteen
  chunks are kept; the 0.7 tuning spans the spawn ring over 58 ticks
  (83·0.7). Charge rises by five sixths of the tick in seconds; past 1 it
  releases with an alert 1800 cm around it, and the chunks fly at 0.96 of
  their motion a tick. Each chunk's collision reach grows 0.4 cm and its
  drawn radius `(speed·0.7 + 3)·0.0006` cm a tick. Two chunks closer than twice the reach
  push apart by half the overlap. A hit body takes `(1 − d²/r²)` of the
  push, a character's limb an impulse of at most 30 doubled, a stun of
  `0.08·(1 − share)` between 0.008 and 0.08 s, and 0.01 of anger toward the
  caster. Its light is tint (0.16, 0.08, 0) at the chunks' mean 120 cm on
  each axis, radius 5000 while charging (1000, or down to
  `5000 − 13333·fade`, once released), each block `light·strength`.
  A chunk's ribbon runs through 12 trail points with 8 samples, in the
  chunk's two random tints; the draw's texture offset is (0, 0, 1).
- **Hits** (both effects): a simulation object (the 0x60000000 class) is
  touched through its capsule and bounding radius; every body point inside
  a chunk's `reach·2 + margin + 40` cm moves along the chunk's last motion
  by `(1 − d²/r²)·0.8·0.025` and gathers `(1 − d²/r²)·0.0001` of impact in
  double precision. A chunk inside a body's sphere is pushed out by
  `radius/|d| − 1`, and a rebound moves the chunk half way to the contact
  and keeps 90 % of its reach.
- **Torrent hits**: a stream's live history points reach
  `age·scale + margin + 10` cm (scale 0.25 against the level, 0.5 against a
  character). A body point inside moves along the point's motion by
  `(1 − d²/r²)·0.01` and gathers `1e−5` of impact; the stream point keeps
  0.5 of the way to the contact; the stream's channels rise by the point's
  remaining life over 66. A character hit drops its guard by
  `inv mass·0.0005` (double), counts the hit, knocks with an impulse over
  one normalised and capped at 30 then doubled, clears a 0x10 stance's flag
  and timer, adds 0.01 of anger, and sets the stun to
  `0.08·(1 − inv mass)` — floored at zero and, the original's inverted
  clamp, to 0.008 above 0.08. A limb takes
  `mass/(mass + 0.5)·scale·0.3·(2 − s)·s` of its overshoot and weight.
- **Torrent light buckets**: a hit point within 25600 cm² of a bucket joins
  the nearest, its position averaged by the bucket's weight and its level
  added; further than that a new bucket opens, until sixteen exist, after
  which the nearest merges whatever the distance. A bucket's level is
  `(2 − charge)·charge`.
- **Aim**: both effects take the planar yaw of the hand's direction from
  the view, limit it to 45° around the body's aim yaw and build the pitch
  from the height. The torrent's contact setup is 0.25 of reach and 0.5 of
  share; the character's is 0.5.
- **Torrent**: sixteen streams are born 16 cm out, their height 1.5 times
  the direction less 30 cm, with a spawn delay of 8 less a random 0 to 2
  ticks and a 66-tick life. The level is `0.4·(2 − charge)·charge` while
  charging; past 1 the streams push 15 cm along the aim a tick. A fade
  drops the level 0.01 a tick and stops below 0.02. A hit point takes 1 % of
  its overshoot as a push, and raises the stream's channels 0.005 and 0.01
  (the second up to 1.8). Its light is tint (0.12, 0.06, 0) at the stream
  heads' mean 120 cm on each axis, strength 2000 and each block
  `light·level·0.25·(2 − charge)·charge·1.2`. A stream's ribbon has 12 quads
  through 16 history points. The cast plays `thmfrc_cu01` and
  `thmfrc_trntdr`, motion 0x26 and the `PowerUsed` event.

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

## The force domes, the wave and the vanish

`exa_game::effects::{force_dome, force_wave, vanish, danger, emissive,
effect_geometry}`.

- **Ripples** (`force_dome::Ripples`): at most eight. A strike is capped at
  5 and goes to the ripple whose target lies within the reach (30 cm plus
  1.25 per ripple), summing its points and keeping the larger strength;
  otherwise it opens a new ripple. Each tick a struck ripple's target
  becomes the mean of its strikes (half way there once shown), its
  strength loses 0.05, its shown value moves toward it by at most 0.04 and
  its place eases 0.05 toward the target; a ripple with both values under
  0.07 is replaced by the last. The Bubble's glow is
  `x = sum(shown)·0.04` through `(2 − x)·x/2`, 0.5 above 1.
- **The Barrier** (`force_dome::Barrier`): a cap at the caster's x and z,
  its height a damped follow of the higher of the caster's height plus 10
  and the ground plus 100. It drains the power's focus rate while growing
  (half while standing). It grows at `0.00125` a ms only while under
  800 ms have passed, so the tick never reaches 1; its drawn radius is
  `(2 − s)·s·0.9·r`. Ending flips the share; it shrinks at 0.002 a ms with
  a drawn radius `(1 − s²)·0.9·r`. Its cap is drawn 85 by 85 by 25 cm,
  75 cm along the facing, bound `√(25² + 85²)`, its glow four times the
  power's hit value. A body part (not the caster's) inside the radius plus
  its own, above half the radius, within the drawn radius sideways and
  outside the inner surface is pushed out and ripples by half the push.
- **The Bubble** (`force_dome::Bubble`): grows over 1200 ms, breaks with a
  sound at volume 0.3 and shrinks over 700 ms. Its caster's parts that
  reach 10 cm inside the surface are pushed back in; other bodies' parts
  that the growing shell covers are pushed out. A struck character is
  shoved 40, plays a hit sound at most every 400 ms and loses balance.
  (The 20 m range is the power's.)
- **The Wave** (`force_wave::Wave`): 32 chunks 80 cm out and 30 cm up, each
  trailing twelve slots. Charging keeps them at the caster, grows them
  0.0015 a tick and eases their trails; past a charge of 1 the blast is
  released at 1200 cm. Thrown, a chunk's reach grows 0.6, its radius by
  0.0006 of its move, its trail by `0.07·move + 1`, and it moves by its
  last move plus `2.4·(1 − charge)²` along its direction, times 0.96;
  close chunks separate, and the effect ends when the chunks' 58-tick
  lives run out.
- **The vanish** (`vanish::Vanish`): hiding clears render bit 4 and the
  shown amounts; showing sets them to 1. The phase hook shows the body
  (phase 4) until the phase table ends, then hides it for good; a lent
  body (from a running constant displacement) is flagged 0x20 and hidden
  again with 0x30 cleared at the finish. The power classes' vanish phases
  hide and show through it (`VanishStep::apply`).
- **What the AI sees** (`danger`): within 12 m the record gains flag 4, the
  urge `min(1 − d/1200, 0.99)`, and from `s = (2 − u)·u` the near score
  `36·s` (blast), `12·s` (wave) or `6·u` (barricade, from the nearest wall
  point), doubled when released (tripled while rising) with the alarm
  raised; the fear is `s²·k·0.5` (blast, `s²·k` above 12), `s²·k` (wave)
  or `u²·k·0.1` (barricade), `k = 1 − relation/100`, tripled when
  released; the face word mixes the viewer's look and facing. A released
  blast within 5 m of its line pushes the viewer `(1 − 0.002·d)·6`.

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

The force and mind passes are described as draw calls in
`exa_game::effects::draw`: program pair, uniforms, textures, blend, depth
and view. The force geometry is one mesh a view drawn with `effectforce_v`
and `effectforce_f` (blast family) or `effecttorrent_f` (torrent), with
`ObjMat`, `TexOfs` (the blast's (0, 0, 1)) and `EmiTex` on unit 0; the GI
lights carry their position, radius or strength and colour blocks. The mind
sight is one draw a tracked body with `TexVecU`, `TexVecV`, `EyeVec`, the
three `MindCol` vectors and `EmiTex`, and the spirit screen one 512² draw.
Destroying either force effect releases its two sound handles and its GI
light and drops the chunks or streams. The actual GL calls are platform
glue and stay out.

The effects' meshes come from their own builders but are drawn with
`exa_render::glow::GlowMaterial`, an additive stand-in: the original's
electricity program, `effectforce` and `effectfbrcd` (with the animated
force noise) are not ported, nor are the lights, sounds, the navigation
stamp or the AI's sight of a barricade (its rules are `effects::danger`,
which nothing registers yet). The domes, the wave and the rain
are not drawn yet (nothing starts them); the GI emissive objects behind
the domes, the bubble, the barricade, the strikes, the tendril and the
teleporter bubble are modelled in `effects::emissive` (their programs,
extents, colours and boxes) but not injected into the voxel volume.
The force blast and torrent carry
ribbon vertices (`blast::ribbon`, `torrent::ribbon`) and
`effects::ribbon_mesh` turns them into the glow material's mesh; no system
spawns them yet, because Exarust has no power bar and the AI does not cast.
The mind-sight overlay's sphere and its animated noise are not drawn either;
its per-entry colours, weights, fade and dwell are in `exa_game::mind`.
