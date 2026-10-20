# `exa_game`

Game modes and session flow: Story, Arena (company management, matches, seasons), Practice; save games; ties all plugins together.

**Phase:** Phase 4–7 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: the arena career (`career`) has the company, the roster, matches, a shop line priced at 1, 12, 144 or 1440 copper with a quartered sale, wages, the season, overnight rest and a trainer's lesson. One effect per thaumaturgy domain (`thaum`) and an Exarust session save (`session`) are in. The name klean selects emblem background 52 and design 81. A practice fight (`practice`) advances by one and stops at 8: novice names opponent01 through opponent08, expert names opponent09 through opponent16, and red gains a tenth of the maximum. The partner is not spawned. An unwatched match draws a team from `(count × 0.5 + 0.5) × strength sum`; mode 16 leaves the player's team out. A bracket clears losing ids. The first group keeps a forced id and does not step the roll. A later group keeps the team whose weights pass the high half of the weight total times the stepped roll. A tournament field draws 16 of the 19 companies. Roll 0 ends at 0x4b157bf0. Roll 1 ends at 0xead32bb1. A day chooses six of seven templates. Rolls 0 and 1 end at 0x1210d4e5. Roll 4 ends at 0xb761a41e. A performance counter replaces the roll by reversing its low 32 bits. A copied match multiplies the fee and the prize by one more than the slot. A seeded unit draw tempers 624 words. Seed 5489 starts at 0xd091bb5c. A record copy writes plain fields, the gaps, and the tail. Managed strings are not in. Before a match the first specialist under 700 health is removed and the death count rises. Health from 700 through 64999 leaves without that rise. Health of 65000 stays. A combatant stays. Kit release is not in. In-game menu command 4 in arena dismisses a child whose flags include 0xce100000. That child releases a capture and clears style bit 0x20 when it holds one. A remaining capture publishes its code and name. Bit 0x8000 passes its low byte as the shape. Shape 0 shows the default image when it differs and is not zero, and stamps the clock. Shape 7 shows its record. The child clears a hover, closes the task's other children that name it, then closes its own children from the first to the last with argument 0, unless it is already closed. After the lists, a focus is counted once before argument 1 copies ten hand words, when a focus is set or the mask shares bit 0. A session mark of 1 becomes 0, a flag becomes 0, and a code becomes 4. Side 0 copies sixteen words from the first bank when its flag is set. A ready install of index 1 shows that record once and runs its hook. Index 0 shows a fallback and stamps the clock. A shown value is passed with two platform words. A shown value activates the action set. The Steam library is not loaded. The gate hook sets its flags to 1. The watch hook stores a stamp of 0 without bit 0x2000, and a leave enters the session. A clear gate counts a pick. A taken item sets flags 5. A hold byte does not pick. A parent equal to the world counts an open. A fast aim copies its place. A miss with the hold byte clears the aimed word. A taken item sets click flags 0x11. A hover stores flags 0x15. A record of kind 0x10010a or 0x10010d whose link is the world counts one open. A different link does not. An item id of 0 does not count a kind call. Bit 0x80000000 searches the high table and bit 0x40000000 searches the wide table, after those bits are dropped. A span that holds 0x20000 counts one kind call. A cursor at the view edge clears the hit. A marked item drops a held count above 0, and byte 1 closes a pointer. The first pane tagged 0xce1c2013 is shown. A scene keeps the last positive body. A new pane doubles the row before the slot and clamps the place. The ray starts at 4000 along z. A centre at 100 leaves reach 100, and a centre at 500 is left. A centre child sits at half the spare. A free camera ray, nested children, and the focus method body are not in. A cleared mark registers code 0x8001 unless the hold byte is set. Raising the mark drops the hooks. An open world sets the side from its byte. Raising the mark to 1 installs index 0 and does not copy a bank. A mask without a focus does not count it. A shown child hides and the redraw word gains bits 0 and 1. A pinned child raises a notice and still closes them. The command then runs that leave, asks for a save that does not abort on low health, marks the hub busy, and sets the phase to transitioning. Drawing the cursor image and the other menu commands are not in. Menus, the other emblem controls, and the original's packages are not.

Story experience (`experience`): new item kinds, kills and explored map
cells (the 5 × 5 cells around the player on a 600-unit grid, the position
rounded down to its cell, a bitmap per
level) earn discovery points, once to the account pool and four times to
the player and each companion. Once a second a character converts a point
per 50 ms of its cost, `4 + skill levels`, into 2 points of its trained
skill; the player's paid cost levels the power in training, a companion's
level adds 10 combat skill and courage and picks its next technique; the
account pool converts at `(global / 5000 + 1) × 4` up to 25000. The
account experience's worth and its inverse, the companions' list, the
fighting estimate and the discovery chunk of a save are in; the arena's
lesson goes through the same skill lump. Nothing calls the tick in a
running story level yet. The shop (`trade`): a first stock of 18 to 20, the
rotation by size, candidates by kind, listing, model and rank weighed by
rarity through the original's quick logarithm and exponential (duplicates
divided by 3, arms halved), the draw, new apparel and weapon copies, the
merchant's category and rank options, and the transaction against the
company's coins (`Company::restock`, `Company::settle`). The shop window,
an apparel copy's description phrase and the weapon variant roll are not
in.

The small effect nodes (`effects`) are in, on the original's game clock
in whole milliseconds: a level portal marker (the `0x3DD0E004` emitter
node, `_PortalPrt` in `uwportal01`) gets the soft blue 0.75 light of
500 cm 50 cm above it when its model loads (its `_PrtlPrt01` sparks are
not simulated), and a body stepping through one gets the seeded
`_PrtlDisp` dissolve burst (cells 6 cm apart, two fifths taken, each
particle's position moved halfway between the node's two frames,
jittered ±1.5 cm and pushed 1.5 % out from the body's centre, lifetime
41 ticks less up to a quarter) with `portaluse01` at 0.6. A lit held item
carries a flame row from 30 to 115 cm with the `_FlameMat01` shader,
fading in by 0.02 a tick and out by 0.015 in double precision, its
ribbon's points closing a fifth of the gap to 20 cm above its ends, lit
by a 5.0 warm light of 800 cm whose softness walk is 0.87–1.15; its
ability word and fuel decide ignition, `FireSizzleA` plays at 0.3 and the
`Flaming` modifier is 0x10020000 (no Exarust item carries that ability
yet, so nothing lights one). A displacement shell opens over 400 ms and
closes over 300 ms from its shown value, with the fade (1 − shown²)², the
texture offset, its fade plane and 900 over the squared distance as its
detail; a teleporter scan opens over 4800 ms, closes over 800 ms and
flashes its colour at three times the shown value when it loops; a bubble
grows over 1800 ms, rests 500 ms or holds and charges over 7000 ms
(flashing at 5600), then shrinks in 400 ms around a 120 cm sphere; the
domain wheel is a 280 px form of six domains (Mind, Force, Body, Energy,
Light, Displacement) clockwise from the top of a 76 px ring, with a
20.8 px hit, its hover, label, open ease, choice flash and confirm rules.
Nothing starts the shell, scan, bubble or wheel yet: their callers (the
displacement powers, the teleporters, the portal screen and the power
tree) are not in. Drawing the flame ribbon's vertices, the shell and scan
mesh copies, the bubble and wheel geometry, the portal particle
simulation and the bubble's GI light values is not in.

It already runs the character ticks of physical bodies, the player's last,
each body's whole tick before the next, in the original's stages
(`stages`: the driver and one module a stage; the rules in `limp`): its
condition, dying tick, falls and get-ups, fall check, then its bearing
with its get-up's first quarter, its clip's progress, walking and off its
feet by its condition (its feet read them), a collapsing body's bearing
saying so (posture; a body that dies there skips the later stages); its walk's
lowest ankle while it moves and its balance against the level
(movement); its clip (the capture of a start
from the body's physical turns, the original's clock rate for what the
body does (the base in its stance whatever its balance, twice it
kicking), the pose it holds while dazed, the sample taken in the
original's per-part pipeline (the Run clip overlaid on the Walk by
0.7 × the run blend, or by the blend sprinting) with identity slots for
the femur posture, the IK chain, the torso twist, the head look and the
foot pitch; a get-up's clip ending into the stance writes the condition
word normal; sampler); its targets, the sampled parts on its character
frame, and its lift, its controller told to follow the frame (carry); a body asked to go limp keeps its motion unless given a
velocity (`limp`),
the original's falls, staggers and get-ups from last tick's balance
(`balance`, in the posture stage after the condition: upright the Defence
stance or Rebalance, leaning a stagger (a get-up jumped in a few keys) or
Fall Forward/Backward, lying still a get-up by the pelvis's front axis
with ±0.2 of hysteresis once its feet give support or 100 ticks after,
turned the way it rises, from the whole lying body, its feet's contact
states cleared; unsteady (0.6–0.8)
the Defence stance or Rebalance; in the movement stage the get-up picked
again under 40 % and the turn toward a fall; a get-up's extra energy in
the readiness stage; a fall leaves the controller, and
`exa_physics::body::Fallen` marks a body off its feet), the
player (`player`: tank controls whose steering starts only the gaits, the
chase camera on the drawn pelvis, combat facing, its combat stance and the
original's attack request each behaviour tick (press, hold, release, the
double click and Alt) through the attack executor, with the clips of the
moveset its hands pick; a dropped request releases the swing, a released
main-side attack returns to the stance once its stall is over, and the
body turns at the swing's rate while it attacks),
falls to death and the kill plane (`fall`: the fall check of a living
body in the falls branch, in its posture stage, its scream and the player's pit sound, the
death fade's start, death and removal from the world; dead bodies below
−50 m taken once, the player's kept),
the world cursor's shape from the player's condition word (`cursor`), the health
bars' values and the low-health heartbeat with its sound, started by a
blow's low-health check on the player's body (`exa_combat::Weakened`;
`hud`), the
held torch (`torch`, which fills its hand and holds it by the grip
table's one-handed or off-hand grip for the holder's size and race; its
tests pin the "Torch" clips' arm on the body model), the per-tick body trace
(`trace`, with the foot contacts' states and anchors and the ankles,
ending with the condition word and the swing's rate, strength and state
word), and the debug tools ([`docs/DEBUGGING.md`](../../docs/DEBUGGING.md)):
the body overlay and joint readout (`body_overlay`, with the feet's
anchors and states), the joint trace
(`joint_trace`), the trace of contacts between body parts (`pair_trace`)
and the simulation controls (`sim_control`, with the scripted pauses at a
tick, a stretched link, the first blood jet, the player's first landed
blow or the first collapse).

## Scope

- App states (boot → menu → loading → in-game).
- Arena: companies, fighters, matches, rewards; Story: levels Underworld 0..8; Practice/sandbox.
- Save-game format of our own (original saves are not a compatibility goal).
