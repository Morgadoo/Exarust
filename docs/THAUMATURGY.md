# Thaumaturgy

The original's powers as Exarust runs them: how a cast is let through and
priced, what each Force, Energy, Displacement and Mind power makes, and the
effects (electric arcs, the force barricade, displacement wisps, the force
blast and torrent) that run as simulations in the game. The Mind domain's
casts and sight are in `exa_game::mind`; the force blast and torrent in
`exa_game::effects::blast` and `exa_game::effects::torrent`. Lengths are in cm, as the original's effects
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
  without the focus item, less one for each running persistent power. Above
  30000 the tier count passes the record's six technique bytes, which the
  original reads past; Exarust scans only the six it holds (a deliberate
  deviation, no out-of-bounds read).
- **Prices**: the row is found by the id under `0x0e0f_ffff`; an instant
  power cast again with the repeat technique costs half; a sustained power
  costs the tick in milliseconds times its cost times 0.001 (Force and
  Displacement) or over 1000 (Energy), half again with the sustain
  technique; persistent powers hold their cost (three quarters with the
  reserve technique) until they end.
- **Settle**: a body starts with focus and capacity 1. Each body tick,
  before its powers run, a share of the reserve's change (what holds add
  and releases take away) moves into the capacity while that change is
  beyond 0.001 either way, the share climbing by 0.001 a tick (to 10) and
  dropping to 0 otherwise. After the powers, with more than 0.0001 spent,
  a share climbing the same way leaves both the spent focus and the pool;
  with nothing spent that share falls by 0.01 a tick and the pool regains
  `√capacity × 0.000301205 × 0.1` up to the capacity, the 0.000301205
  half again with technique bit 0x20_0000 and plus any effect's bonus
  (one sets 0.000100402). Either settle restarts a regeneration ramp,
  which otherwise climbs by 0.001 of that rate to at most the rate.
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
  waits on the hand for its pick while it runs (a press that would lift a
  prop hands it the prop, and the pick alone answers the reach: a point
  it can take is in reach wherever it is, any other out of it),
  holds a pick at a third of its extents' sum and aims at its centre.
  Each tick, when the cursor's ray (passing over the held body) hits
  within 40 m, the aim is that hit raised `100 − depth` (at least 10) back
  along the ray and, when the line from the caster's head to it is clear,
  settled by `h/(|h| + 1)·200` of its height over the caster (`h` that
  height times 0.005); the held part's origin is pushed at its reach (half
  its count of linked child bodies plus one; Exarust counts the held part's
  child parts) along the unit direction to the aim
  (the direction times 0.02 within 50 cm or for a distance that is not a
  number) less a fifth of its own motion, each particle by its origin
  weight, and the body wakes. It turns by `reach²` times the double 0.001
  a tick by the stick or the key pairs (each pair if/else), about the
  camera's right axis and the vertical, and stamps its user for 2 s
  (`exa_game::thaum::kinesis_live`); Throw
  takes the body a charging kinesis holds, else the first body the
  cursor's ray meets passing over the level: a dead character, or a prop
  whose mobility widened to double is at least 0.01 (a single 0.01 is
  not), that the hand does not hold, and plays `thmfrc_knsst` there (0.3,
  pitch `0.8 + 0.4·roll`). It holds no reserve: its lift aims 100 cm
  above the body and spends `tick·cost·0.00125` every steered tick at
  `mass/(0.1 + mass)` of the unit direction (less a fifth of the body's
  own motion), drops the shell after 800 ms and coasts at six times that
  share toward the caster's aim point (the player's cursor; ending at
  once within 100 cm), and restores the materials 200 ms later. A body
  handed from a kinesis is lifted at its centre and spends nothing a
  tick: that start never stores the cast's cost. Every lift plays
  `thmfrc_throw` at the body (0.6, pitch `1 + 0.2·roll`), a one-shot that
  fades when the throw ends before its drop. Each tick moves the body's
  first part, each particle by its origin weight, and wakes it; a body
  gone ends the throw (`exa_game::thaum::throw_live`).
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

`thaum::cast` (the cast plugin) creates the classes from the player's
power bar: `C` opens and stores the bar, a digit picks a slot, the arrows
step the choice and `Q` hands the picked slot to the bar's cast entry,
which takes the slot's word as it stands, runs its domain's
front end by the word's domain byte (Force, Energy or Displacement; a Mind
id is refused until the Mind target pick is wired) and makes the class the
cast's creator makes. Each body tick walks the active list in the
original's order, passes control event 0 (event 2 once a power is told to
end), releases the focus a finished power held and takes it out with the
last entry in its place. The live arcs, barricades and wisps are spawned
with their powers and removed as they end; the other effects have no
entity yet, so their classes keep running. A blast or bolt cast makes its
blast entity (`thaum::blast_live`): its start plays cast motion 0x22 with
the setup's power id and, when that motion starts, takes the caster's cast
hook, which answers −1 and fizzles a charging blast when it is called (a
later cast's start, or the release flag, which also ends the cast). It
pays its drain each charging tick whether or not it holds the hook; a
focus shortfall with a hook set goes through the shared cast end (the
hook, the hook owner and the power id go, and a condition word of exactly
casting, 0x20, becomes 0 and the caster returns to its stance over 30
frames in the same tick), and fizzles the blast only while the hook
is its own. The throw clears the hook and the power id itself and the
cast motion plays on. Each tick the
blast reads the caster's right hand, facing and aim point (the AI's
target's chest or the player's cursor), and a thrown blast pushes the limb
spheres its chunks reach and knocks a struck character by its gathered
impulse; that sphere test, the body's mass as its parts' sum and the knock
as a knockback step are Exarust's. A barrier cast makes its dome entity
(`thaum::barrier_live`): the creator hands it the power's cost as its rate
and the radius 100, its start plays cast motion 0x23 with id 0x10205 and
takes the hook, which ends the dome (state 2) and answers −1. Under state 2
it pays the rate a tick, half once standing, and a shortfall calls the
hook and ends the cast; it pushes other bodies' limb spheres out of its
shell and opens ripples where they strike, and a struck character is
shoved 40 cm (a knockback step here). The cap's frame (the dome's centre,
z along the flattened facing) and one contact a body are Exarust's; the
contact's balance drain and knock count, the sounds and the draw are not
wired. A bubble cast makes its dome entity (`thaum::bubble_live`) at the
cast place, 120 cm across: its start plays cast motion 0x24 with id
0x20207 and takes the hook (0 to a call carrying bits 0x28, so a cast's
start, event 0x20, is refused while it is set); the grow passing 1 clears
the hook and the power id (not the cast end: the cast motion plays on),
and the
power's ending (cast again, or the dome 20 m from its caster) breaks the
dome, which shrinks over 700 ms and leaves. It holds its caster's limb
spheres in and pushes other bodies' parts the shell covers out, with
ripples and the dome's 40 cm shove (a knockback step here). A wave cast makes its wave entity
(`thaum::wave_live`) round the caster's body root: its start plays cast motion
0x24 with id 0x204 and takes the blast's hook, so a later cast or the
release flag fizzles a charging wave; it pays the cost times the tick's
seconds each charging tick, after its chunks move and before the charge
step, and a shortfall fizzles it and ends the cast through the shared
cast end. The release clears the hook and the power id itself, the cast
motion playing on, and a thrown wave pushes other bodies' limbs through
the blast's contact arithmetic (its chunks keep no wear, an Exarust
reading). Its alert (1200 reach, within 18 m, under 150 cm of height), sounds and
draw are not wired. A torrent cast makes its torrent entity
(`thaum::torrent_live`) at the caster's right hand: its creator gives the
cost a second and id 0x10203, and its start plays cast motion 0x26 and
takes a hook answering −1 that fizzles a charging torrent and ends a
running one. Running keeps the hook, so the torrent pours until a later
cast, the release flag or a shortfall (the drain while charging, half of
it while running) calls it. Its delays draw from the effects' shared roll,
its running points push other bodies' limb spheres (each push costing a
character the torrent's energy loss), and it leaves the scene on the first
frame that would build no triangles: fizzled, or no stream with more than
two samples. The turned aim (the blast aim here) and the sphere test are
Exarust's; the character knock, sounds, light and draw are not wired. A cast motion is played through
the animator by its action name (0x22 Force Blast … 0x28 Disp Form),
looping when its clip's mode has bit 0 (Force Barrier, Force Torrent). A
cast hook answers by the event's bits (`casting::CastHook`): the engine
asks it with 0x20 when another cast starts (0 or more refuses that cast),
0x80 when the cast's focus falls short, and 4 when a motion replaces the
cast motion, which a one-shot cast clip's end does as it returns to the
stance; a negative answer to 4 drops the hook and the power id
(`cast::call_motion_hooks`, for any action left other than the walk, the
run and the attacks). The shell's raise, Force Strikes and the barricade
install a hook answering 0 to bits 0x29 once their cast motion starts,
and the bubble's answers to bits 0x28, so none lets another cast start
until its cast clip ends (a strikes cast whose motion is refused makes no
strikes, though it still answers 1). Kinesis's hook keeps events 8 and
1 and the throw's event 8; any other event, a focus shortfall included,
ends that power. A throw from a kinesis ends the kinesis without ending
the cast, and the kinesis's hook then also keeps a cast's start, so the
throw plays on in the kinesis's motion. Shock's hook answers −1 and,
called while the shock charges, clears it so it never fires; Shock
Weapons, Constant Displacement and the Summon keep bits 0x29. The Long
Door's hook answers 0 to everything, and Vanish's does until its hold.
From the hold on, an event with any 0x120 bit lets the vanish go and
answers −1; any other event with a 0x91 bit lets it go and answers 0
(`CastBody::hook_asked` records the events a hook was asked with, for the
hooks that act whatever they answer). A sustained tick
whose focus falls short with a hook set ends the cast, as the engine's
spend does. The blast, wave, torrent and barrier
hooks answer −1 to everything, and their effect fizzles or ends when
called. The player's controller asks the hook every tick
(`casting::player_hooks`): with 1 while the left button is down, with 8
outside combat mode, and in combat mode with the body casting with 2
while the right button is down or a gamepad stick over 0.1 is pushed with
Sprint held (else 1 again); a negative answer ends the cast. The combat
branch is skipped while the body's tone is under 0.85, as the original's
behaviour tick does. [inf: the original's game-view and status-bit 0x100
gates around these asks are not reproduced; Exarust runs the asks for a
living player whose caster it ticks.] A casting body asks no attack. An attack start (the player's,
past its energy check, or a duellist's) asks a set hook with 0x100 first
(`exa_combat::AttackHook`, kept by the power system): an answer of 0 or
more refuses the attack, a negative one ends the cast and the attack goes
on. So attacking during a Long Door or an early vanish is refused (and
lets the vanish go). So outside combat mode a shock,
a blast or a torrent ends on its first tick, while kinesis, the throw and
the 0x29 hooks hold. With a cast motion playing, the body tick's release
flag (pressing a running cast again) asks the hook with 0x10 and ends the
cast whatever it answers (`casting::release_cast`). A duellist's combat
state, a second after its cast began with the body still casting, asks
the body's power id's domain evaluator and, when that answers nothing,
the hook with 0x10; a negative answer ends the cast and the AI lets it
go. Before its state the AI lets go of a cast whose body has not read
casting 500 ms after it began. The
player swap, the spirit body's character generation and the effect meshes are descriptions here, not
spawned entities; the kinesis's held material, pick shell and pick
sound, and the throw's shell and material, are not wired. A kinesis pick,
and each tick a throw coasts after its shell drops, stamp the body with
the caster for 2 s (`thaum::user_stamp`): a limb the body hurts while the
stamp holds adds the script's concern toward the caster (amount 0.9, gain
three times the damage) when the caster is one of the struck brain's
contacts.

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
focus, facing within 0.5, a free slot). Each creator's material
(`_ForceMat01`, `_ForceBarrier01`, `_ForceAura01`), its parameter block,
the shared `gi_multipoint` emissive program, the animated `_ForceNoise`
field (one 8-bit channel from `exa_render::image_noise`) and the sounds each one loads are data in `force` (the GL scene
nodes, shader binding and GI registration are not issued).

## Mind

`exa_game::mind::casts`. The table has sixteen rows: Sense Minds 0x101
(0.13) and its persistent form 0x20101 (0.27), Mask Mind 0x102 (0.12) and
0x20102 (0.24), Spirit Form 0x10106 (0.025), Calm 0x103 (0.14), Enrage
0x104 (0.16), Confusion 0x105 (0.17), Trust 0x109 (0.19), Loyalty 0x2010A
(0.29), Suspicion 0x10B (0.19), Control Mind 0x2010C (0.35), Command Dead
0x2010D (0.25), Fear 0x107 (0.18), Embolden 0x108 (0.18) and Raise Dead
0x10F (0.30). The persistent Sense Minds and Mask Mind, Loyalty and Command
Dead can be rebuilt from a save: the row's cost goes into the reserve
(three quarters with the reserve technique) and the power holds the whole
cost, which the load then takes three quarters of with the technique.

- **Front end**: while the body runs Control Mind only Control Mind may be
  cast, and on the held body; the row is priced as the shared front end
  prices (the tick over 1000 for a sustained row).
- **Gates**: most casts first need focus of 0.3 times the cost (in double
  precision; a NaN passes), then a picked target. Record kinds 1 and 2 are
  living and 4 and 5 undead; anything else refuses after whatever the cast
  already spent. An undead target takes the cost (half for Control Mind)
  and answers 0 with the cast's sound. Suspicion needs the whole cost
  first; the persistent casts, Loyalty, Control Mind and Command Dead need
  a free slot (the body's slot count, less one a running persistent power;
  a running Spirit Form leaves Control Mind none) and the whole cost.
- **Calm**: every contact's concern and hostility settle into its regard:
  more than 10 loses 10 and gives 30, otherwise (or a NaN) it all goes in
  three times over. Alert drops 10, stress 50 and fear 1, each floored at 0
  (a NaN stays), and the calm factor becomes 1.
- **Enrage** adds 50 alert and the timed enraged bit 0x80000; **Confusion**
  adds 50 alert, clears every contact and the AI's level and focus, and
  sets the timed confused bit 0x40000. **Trust** places its timed marker
  0x100109 toward the caster (refused on a body already loyal to it) and
  halves the target's contact toward the caster, its regard set to 30;
  **Suspicion** places 0x10010B. **Fear** adds 20 fear (40 at most),
  **Embolden** 50 courage (80 at most).
- **Loyalty** on a body already bound to the caster ends the binding (2);
  bound to another caster it refuses. Otherwise the target takes the
  marker 0x10010A with the strength `((relation + 100)·0.005)²·100` (the sum
  in single precision, the rest in double), its contact toward the caster
  settles (regard 30, no concern or hostility) and its AI re-arms; the
  caster runs Loyalty, which takes the cost.
- **Control Mind** cast while it runs ends it (2), unless it is still
  starting; a target with technique bit 0x10000 resists (half the cost,
  `ThmMnd_E02`, −1). The binding itself spends its cost as it runs.
- **Raise Dead** and **Command Dead** need a collapsed body (condition
  0xA00) that has not been raised (rules bit 0x10) and whose character
  allows it (bit 0x40 refuses). A body with no decay is revived: its decay
  becomes 0.2 (the original stores it and then tests it, so the bare-body
  branch never runs), its gait scale `1.25 − decay²/4`, its pace
  `round((1 − decay/4)·1660/83)` = 19 ticks, its most energy `1 − decay/10`
  and its stamina `(1 − 0.3·decay)·(1 − 0.2·load)·80·build`. Either way its
  yellow and red health become 0.5 and 1. Command Dead also places the
  marker 0x10010D and runs the caster's power; cast on a standing body the
  caster commands, it ends the command.
- **Sense Minds and Mask Mind**: the instant Sense lasts 4000 ms, or the
  focus's share of it (rounded to even), on the real clock, and shows the
  overlay that long; the instant Mask stamps its class with the scene clock
  and sets the masked bit. A persistent cast ends the running one (2) or
  ends the instant one beside it and starts its class.
- **Spirit Form** needs 0.05 of focus (double precision); a running form
  ends (unless its screen is still coming up); otherwise the form starts
  with the cost as its drain, the spirit status bit 0x100000 is set, and the
  spirit screen, its music (`Msc_SprtRealm` at 0.1, 0.15 with the music
  option) and the spirit body are asked for.

What the Mind domain shows is in `exa_game::mind::sight`:

- **Target pick**: a character on screen, not the player, not hidden (rules
  bit 0x10), not collapsed unless the dead powers ask, seen by the player's
  view unless Sense Minds runs, within `height·zoom·0.5·40/depth` pixels of
  the screen centre (an unordered compare passes); the nearest share wins.
- **Overlay**: a start registers its frame hook when off, shows it for the
  duration (to the end of time for the persistent power) or extends a
  showing one, and restarts its alpha unless it is fading; it fades in and
  out over 2 s by the time since the tracker's last stamp and, faded out,
  drops every tracked mind.
- **Tracker**: at most every 4 ms, every character but the player and the
  body it controls. A living (2) or undead (1) kind gets an entry (a freed
  slot is reused) whose three values follow `mood_values` by
  `exp(−ln 2·dt/1000)` (a half past a second): anger `a = 0.2·anger` and
  fear `f = 0.2·((dread + fear) − courage)`; a collapsed body or `f > 1`
  gives (1, 1, 1), else the larger of the clamped two, `f/(f + a)`, and the
  threshold (larger + 0.2, raised to 0.6 of arousal, set to 0.015 of stress
  when below 0.02 of it, at most 1). A collapsed body fades over 5 s and an
  out one over 3 s from when it was first seen so, then is dropped.
- **Colours**: `MindCol0` (0.1, 0.4, 0.6), `MindCol1` (0.8, 0.6, 0) with
  alpha `(1 − first)·0.6 + 0.1`, `MindCol2` from (0.4, 0, 0) toward (0.31,
  0, 0.78) by the third value with alpha `(1 − second)·0.6 + 0.1`; undead
  (0.3, 0.3, 0.3), (0.3, 0.3, 0.3, 1) and (0.5, 0.5, 0.6, 0.5). The overlay
  gives `MindCol0` its alpha less the entry's fade, the glow `1 − fade`.
- **Billboard**: 15 cm above the head node's height offset, the texture
  axes the node's third and first axes made square to the view, blended by
  `1 − |dir·axis₀|` and 0.025 long, in the node's frame; the node's rows
  scaled by the record's three scales.
- **Glow**: a sensed body's entry (first value 1) that fades in over a
  second and out over the next two.
- **Sphere**: 16 segments of 15 cm, shaped once: x narrowed to 0.8, moved
  by half its negated projection on (0, −0.8, 0.2), tapered by `1 − z/100`
  above its middle and lifted 2 cm. The overlay and glow textures are 256²
  noise (four octaves, 0.25), the spirit screen's 512² (0.8).

## Force blast and torrent

`exa_game::effects::blast` and `exa_game::effects::torrent`.

- **Blast** (and the bolt, through `BlastSetup`): sixteen chunks on the
  icosahedral directions (+z, a ring of five 26.565° up from the xy plane
  at 72° steps, and the ten midpoints), each 16 cm out in the hand's frame
  (its height 1.5 times less 0.5), with reach 4 and a twelve-point trail.
  The aim from the hand's node to the AI's aim point is held within 45° of
  the facing. Charging, the chunks follow the hand and grow
  `0.0015·swell`, the trail eases a tenth toward 9 cm back along the aim,
  the caster pays the drain and the charge rises five sixths of the tick;
  past 1 the characters within 1800 cm are alerted by
  `0.25·(1 − (d/1800)⁴)·(aim·dir + 3)`, the release directions are pushed
  `forward` along the frame's z, the release sound plays and "PowerUsed" is
  raised. A fizzle loses twice the rate and removes the blast below 0.
  Thrown, a chunk's reach grows `growth`, its radius
  `swell·(0.7·|move| + 3)·0.0006` and its trail `(0.7·|move| + 3)·0.07 + 1`
  (double precision); it moves by its last move, plus its release
  direction times `weight·(1 − charge)²·2.4` under a charge of 1, times
  `retention`; a worn chunk spends `round(|move|²·wear)` life and gains
  0.25 reach; two chunks closer than twice the first's reach move apart by
  half the overlap (coincident chunks give NaN, as the original's do). The
  chunks live `round(83·life)` ticks. The frame's texture offset is
  `(0, r₀, texture_scale)`, its charge sound `0.4` of the charge's ramp, its
  release sound the setup's release volume.
- **Blast hits**: a limb takes `move·f·(−k)·0.2` (`k =
  (−inverse/(inverse + share))·push`, `f` its velocity's share of the
  move, clamped to 0..1) and moves the chunk by a quarter of its share of
  the contact less a twentieth of its move; the share and push are 0.02
  and `character_push·2/mass` on a character, 0.2 and `body_push` otherwise.
  A struck character loses `k/1000` energy above 0.25, is knocked by its
  gathered impulse (capped at 30, doubled), gets a stun of `(1 − k)·0.08`
  (0 below 0 and, the original's quirk, 0.008 above 0.08), hostility 1 at
  0.01 toward the caster and a recoil of 5. A simulated object's point
  moves along the chunk's move by `(1 − d²/r²)·push·0.025` and stirs it by
  `(1 − d²/r²)·0.0001`.
- **Torrent**: sixteen streams of sixteen points (up to 24 streams), born
  at the same directions 16 cm out (height 1.5 times less 30); a point
  lives 66 ticks. The start puts each stream's first points 18 and 9 cm
  along the aim; charging feeds the head 15 cm and the point before 30 cm
  out from the birth place; running, each stream spawns a new head after
  `8 − roll(3)` ticks along a direction drawn round the aim, every live
  point ages, pulls its neighbour by `0.1·(1/2 − 2500/(d² + 2500))`, moves
  on by 0.999 of its last move, and the stream splits where two points lie
  more than 1000 m apart. Dead streams are removed as the original removes
  them (it moves the record before the last into the hole, losing the last
  stream). The sounds follow `step_torrent`. The frame's texture offset is
  `(0, scroll, 1)`; its samples feed up to sixteen light points that merge
  within 80 cm (or the nearest within 160 cm once sixteen are kept).
- **Torrent hits**: a point reaches `age·scale + margin + 10` cm (0.25
  against the level, 0.5 against a character), keeps the ease share of the
  way to the contact and raises the loop sound's volume and pitch by its
  life share; a limb takes `k·f·straightness` of the point's move plus
  `(contact − point)·(1 − straightness)·(−k)`, `k = inverse/(inverse +
  0.5)·push·0.3·(2 − s)·s`; a character as the blast's, with `k/2000` of
  energy. A simulated point moves by `(1 − d²/r²)·0.01` of the move.
- **Lights**: one multi-point light each, coloured (0.16, 0.08, 1) and
  (0.12, 0.06, 1) (`effects::emissive`).

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
once). Vanish pressed while it runs answers released, starts nothing and
asks the body's hook event 0x10 (its answer ignored), which the vanish's
own hook takes as its end on the next tick. A constantly displaced body lets a blow through when a percentage
roll is at most `100 >> (s + 1)` (s ≥ 0) or `100 − (100 >> (1 − s))`
(s < 0) for its streak s: 50 % at first, 25 % after a pass, 75 % after a
miss. A passing weapon loses its collisions for 500 ms and leaves a wisp
at the contact. The AI's long door and summoning words carry their cost.

## A constantly displaced body

`exa_game::thaum::veil`. The body-side half of the constant displacement:
the shell (`_EffectShell01`, fade from zero, node flags `0x21000000`, its
end event zeroing the fade and asking the node to play `0x10`), the collision
bit `0x4000` every physics node gains, the `thmdsp_cns01` hum that starts
`0.8 × length × r` seconds into its loop (computed in double precision,
silent at first), the left-behind image gathered
from the body's mesh and skin nodes and ordered by its subobjects' `0x40`
flags, and the visibility bits (`0x10` shown, `0x20` hidden, `0x4` fading,
`0x8000_0000` dead) that hide a part with subobjects by setting its low
hidden flag. The hit veto plays `thmdsp_phs` at pitch `0.9 + 0.2·r`, sets
`0xC000` on the struck node and every part below it, lists it for 500 ms and
leaves a wisp at the contact; each returned weapon ends the shell. A shell's timed segments show
`clamp((elapsed − start) / (end − start), 0, 1)`, inverted unless the
segment is reversed; a segment draws on its side of one half.

## The power tree

`exa_game::thaum::tree`. The tree screen reads a domain's `.pwr` file
(`pwr_mind`, `pwr_force`, `pwr_body`, `pwr_energy`, `pwr_light`,
`pwr_displace`), marks each node from the character's known pairs, and
opens a node whose links all end on learnt nodes. A click on a shown but
unlearnt node marks it as the new power when the character has the
techniques it asks for (a persistent power wants Concentration's technique
2, a power with a high nibble that technique among Insight's; either skill
funds one level for each 5000 points). A click on a learnt node picks or drops it: a pick needs the
domain's cost within the record's remaining focus (4.0 less the picked
powers, each domain discounted by the character's specialisation) and every
link end picked; a drop is refused while another picked node links to it.
Closing replaces the domain's pairs with the learnt and picked nodes. The
view's zoom is a charge of −9 to 5 levels (0.25 to 2.0), its opening height
over 2300 picking the nearest; the pan and zoom ease by the original's
springs.

The power bar holds ten slots; the chosen one carries bit 20. Loading it
from a record copies the words (clearing bits 23 and 24 and the technique
nibble), resets each state and marks a slot picked when its pair is; a
slot's stamp and duration are kept. A slot flashes in over 200 ms, stays
lit, then fades over its duration. A press selects a slot, a dropped power
fills the slot (a drag from another slot leaves a copy there) and a drag
that misses clears its source. The cast entry reads the record's slot (the
named one, else the first marked chosen), ends the body's control instead
of casting while it controls another body, refuses a body whose item flags
refuse powers, and dispatches by domain to the mind, force, energy or
displacement front end. A front end that answers "not now" queues the
slot; a start stores the bar back with the chosen bit moved to the slot,
closes the bar, flashes the slot (from where its state stood, back-dated by
the elapsed share) and stores the last instant power; a refusal plays the
failure sound. The running game does not play that sound or retry the
queued slot yet. Opening the bar loads it and runs a 900 ms rise (the
height and size ease in over the first 400, the pitch eases twice);
closing runs 900 ms, the marked slots falling as `1 − f⁴` and the rest as
`1 − 1.5f`, then clears the slots' flash and picked bits. The bar's slots
step their own flashes (200 ms to lit, then a fade over their duration). The running-power budget is the highest Concentration
technique of 2 to 4, one less without the focus item; powers past it are
told to end. The grant routine (`grant_power`) adds a power to a record
that holds Insight's seventh technique when its node's first link names a
learnt prerequisite, over the first matching pair or appended, with flags
`0x9fff`; the caster's `PowerCaster::grant` is its caller, with no script
or game function offering a power yet. A saved character's power list
(`exa_formats::power_tree::read_saved_powers`) is re-validated by
`validate_saved_power`: each entry takes the flags of the last record pair
it matches under `0xff0f_ffff`, and the saved copy of a power is added with
`0x9fff` when it is not the first entry and its first link's prerequisite
entry is learnt. No character save loader exists yet, so that reader and
rule are tested functions.

## Electric arcs

`exa_game::thaum::arcs`. Tendrils of points grow from a source a segment
(4·size) at a time, open branches where they bend and lose points where
they fold, trade tails with branches that outlive them, jitter (one tick
in 30 they jump), keep 0.9 of their motion and stay within half a segment
of their neighbours' middle, end at the level where a segment meets it,
and shock the first character they touch (once per character with the
shock-once flag; each segment but the last is given to the character's
own query with a 20 cm value), reaching a branch toward it. The emitter
also enters and leaves the world on its node (its hum and its light), and
a node under a local-frame parent takes the parent's matrix with its own
offset carried through it. The character query itself is not ported: the
running game tests each segment against the parts' spheres. Each frame the
light flickers within 0.75–1.2,
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

## The shaped-force aura

`exa_game::thaum` holds the force-shell aura (`ForceShell`, its copies
and uniforms, and the draw in `shell_draw`): it opens over
1.5 times its 800 cm size (1.2 s), holds, closes over 800 ms from wherever
it is and then leaves. A hit keeps at most eight ripples; each ripple's
strength loses 0.04 a tick, its shown amount moves within 0.06 and −0.04,
and it leaves under 0.05 both. The impact sound plays at most every 400 ms
at volume `strength·(0.05 + 0.05r)` and pitch `0.5 + 0.4r`. The shader is
`effectshell_v`/`effectshell_f` on texture unit 3 with `ObjMat`, `LclMat`,
`TexOfs`, `ViewPos`, `Fade`, `TexPos`, `ColPt`, `FdPln`, `ColourInt`,
`TexScale` and `EmiTex`; the fade is `1 − shown`, `LclMat` is the identity
for a rigid copy and the inverse node frame for a skinned one, and the
fade plane rises from the bottom of the node's box (the skin's for a
skinned copy) in units of its height. A rigid copy pushes every vertex out
by twice the unit sum of its faces' normals and keeps the visible
sections' triangles; a skinned copy keeps the skinned sections' vertices
and places each at its place plus twice its normal every frame. The light
is (0.14, 0.08, 1) times a tenth of its curve, and the hum's volume and
pitch follow the opening and closing. The aura is not spawned by the
running game yet.

## Displacement wisps

`exa_game::thaum::wisps`. Rings of eight nodes per strand, 40 cm apart,
rising from anchors on a phased body (per bone) or from a blow's contact;
each node darkens with age, grows 0.1 a tick and drifts on; the strands are
drawn as smoothed camera-facing strips.

## In the game

`exa_game::thaum::live::ThaumEffectsPlugin` runs live arcs, barricades and
wisps after the bodies: arcs follow their hand, end at the static level and
shock other fighters (their parts' spheres stand in for the character
query): a shock lands once the game clock is within 150 ms of the
fighter's next shock time, which then moves 500 ms plus a draw below
200 ms of the engine's generator ahead, and a fighter that is not
collapsing or dead loses the strength in yellow health and takes the
shock-stagger flag, which the next body tick spends by setting energy to
its most minus 0.5 and starting Rebalance with no blend until the clock
passes that next shock (the release, the stagger step and the sounds stay
unwired); a barricade pushes
the particles of every body part it holds (`Ragdoll::push_sphere`), wakes
the body for 32 ticks, knocks a character back a step and sets its
instability, each part it strikes in front lowers the character's energy
again while it is above 0.25, and its line is stamped on the navigation
grid's penalty until the wall falls;
wisps ride their parts. The running game opens the tree with `P` and the bar with `C` (`hotkeys`),
routes the pointer through the tree's hover/click/drag/zoom rules and draws
the tree, bar, tooltip and preview with the six fragment programs in
`exa_ui::tree_draw` and the `.rfi` sheets (`power_screen`). The bar's keys
(`hotkeys`) go to `thaum::cast`, described with the classes above, which
runs the chosen slot through the Force, Energy and Displacement front ends
against the player's body. A duellist whose record knows powers casts too:
before its combat state runs, each record slot is answered by its domain's
AI evaluator (distance and bearing to its target, the target's condition
and status, the caster's focus as the pool), and the power the combat
state's chooser picks starts through its domain's front end as an AI
caster, with no cursor and its hands free. A duellist or placed
character's caster takes its body's technique word from the record's
learnt techniques, and its persistent slots from Concentration's highest
Continuity tier ([formats/RFC.md](formats/RFC.md), Techniques and perks);
the player's caster keeps technique word 0 and one slot. A caster's running shell and
Force Strikes put their factors on its fighter (`exa_combat::PowerFactors`):
a blow it takes keeps 1 − 0.5 of its impact, and a blow it deals is
multiplied by 1 + 0.5 · 1. Every caster's body tick runs the two focus
settles around its power walk, so spent focus drains out of the pool and
then regenerates, and the player's HUD focus bar shows the pool and its
capacity. The bearing measure (1 less the
cosine between the facing and the line to the target) is Exarust's. The
tree screen and the caster share one record:
the screen owns it while the tree is open and the caster takes it back when
the tree closes. The constant displacement's veil, the grant routine and the
tree's key handler are not wired. The
`--debug-arcs`, `--debug-barricade` and `--debug-wisps` flags still put
those effects on the player ([DEBUGGING.md](DEBUGGING.md)).

### Drawing

The power tree's drawing is `exa_ui::tree_draw`: the six immediate
programs with their exact sources, uniform sets and textures, and the
empty-slot, icon, frame, glow, flame and ring geometry. The helpers set
only their own geometry and `par0` (and `tscl` for icons); the caller sets
the colour, so the byte colours are the original's (a float times 255
rounded half to even, saturated, while the alpha byte keeps only its low
eight bits). The tree pass runs in the original's order: the black
background (alpha 255 on the creation screen, else half the window), the
links (a marked source draws bright, a marked destination soft, neither
not at all), the icons, the technique and frame marks, the rings of the new
nodes (the newest fading in over the learning time while the one it
replaced fades out), the picked rings in the domain's colour (a flash
swells the ring, a refusal dips it), and the glows (dimmed to a third
unless picked, swollen by the flash, their alpha by what the node is);
the bar follows unless this is the creation screen. The bar's icons take
the opening and closing fades, group by domain and start each group at its
first slot; the glow of each slot, the flame of the chosen one and the
empty slots' rings finish it. A slot's flash is a signed time difference
over 200 ms and its fade a double-precision ratio over its duration.
Not modelled: the link ribbon's spline and texture (a plain quad strip
stands in), the layout and text of the tooltip, the prompt and the drag
preview, the screen's show and hide fades, the bar's opening and closing
animation in the running game (the pose is implemented but the view always
asks for a steady bar), the bar form's flags and opening stamps, the
running-power mark callback on load, the persistent-pick bit of a slot (derived from
the picked nodes) and the icon radius output. The running screen
rasterises those batches in software (`power_screen::render`) with
each fragment program's arithmetic and the loaded sheets; the shader
bindings of the Force and Displacement effects are data in
`thaum::visuals` (materials, blocks, `gi_multipoint`, the noise picture and
the per-pass texture offsets). The EffectShell, EffectShell pass selection
and the tendril geometry are in `thaum::veil`/`thaum::visuals`.

The effects' meshes come from their own builders but are drawn with
`exa_render::glow::GlowMaterial`, an additive stand-in: the original's
electricity program, `effectforce` and `effectfbrcd` (with the animated
force noise) are not ported, nor are the lights and sounds. The arcs
upload their ribbon's UVs. The barricade's navigation stamp is applied to
the grid (`exa_world::nav::stamp`); the AI's sight of it is still
`effects::danger`, which nothing registers yet. The domes, the wave and
the rain are not drawn yet; the GI emissive objects behind
the domes, the bubble, the barricade, the strikes, the tendril and the
teleporter bubble are modelled in `effects::emissive` (their programs,
extents, colours and boxes) but not injected into the voxel volume.
The force blast and torrent are not drawn: their ribbon vertex builds
are not ported. A cast blast or bolt runs as an entity, without its draw,
sounds, light, alert or stun. Their geometry classes (`effectforce_v` with
`effectforce_f` or `effecttorrent_f`) are in `effects::effect_geometry` and
their multi-point lights in `effects::emissive`. The mind-sight overlay's
sphere and its animated noise are not drawn either; its pick, tracker,
colours and billboard are in `exa_game::mind::sight`.
