# TMotileDynamics: routine map

The character body class of the original engine (`TMotileDynamics`, 100 routines,
123,719 bytes) mapped routine by routine to the Exarust items that reproduce it. Routines
are named by role and numbered by their position in the research pack's function index
(`routine #N`). Sizes are the routines' code bytes. Rust paths are relative to `crates/`.

**Status.** *done*: the logic (constants, branches, float order where it matters) is
reproduced; *partial*: part of it is, with the estimated share in brackets and what is
missing; *missing*: nothing reproduces it; *n.a.*: runtime-library or ownership glue that
Rust does not need (string release, destructor, clone, scene list insertion). *Live* says
whether the running game calls the Rust item ("no" = tested but not called).

The audit was made on 2026-10-29 from the pack's decompiles and assembly and a sweep of
the crates; shares of partial routines are estimates, not instruction-level diffs. Updated the same
day after the character-init presets, the grip and block-stamina re-check and the balance
routine's foot sounds (see Corrections), after the wiring lane (live column), and after the
kernel lane (the damage routine's pain sounds, the feet routine's modes, the init's golem links
and leg order, the standing body's drive), and after the sampler lane (the samplers' heights, the
pose-only key pick, the placement arithmetic, the down body's probe, the living hunch), and after
the muscles-off lane (that sampler's bracket, overlay, height target and queued switch, and the
capture of the current targets), and after the feet-modes lane (the down body's probe and the feet
routine's flat-floor and hovering modes wired), and after the reach-and-jolt lane (the get-up legs' reach,
the flat-floor setter, the player's jolt and rumble, the swing's first clash ring, the dead fall probe
checked unreachable).

## Totals

| status | routines | bytes |
|---|---|---|
| done | 53 | 21,549 |
| partial | 39 | 100,371 |
| missing | 3 | 903 |
| n.a. | 5 | 896 |
| all | 100 | 123,719 |

Weighted by the partial shares, about **98,140 of 122,823 needed bytes (80 %)** are reproduced (n.a. routines left out; counting the partial routines' shares, 76,591 of their 100,371 bytes). Counting only what the running game calls, about **84,030 bytes (68 %)**; the rest is tested but not wired. (Totals as of the round-127 rebuild below; earlier lanes' additions are in the paragraphs that follow.)
The wiring lane made Break Tangle (#38, 370 B), the swing voice of #34 (about 450 B) and the held-voice fade in #49 (about 100 B) live (about 80,400 B). The kernel lane then added about 1,670 B reproduced (the pain sounds and player jolt of #8, the feet modes of #50, the leg order of #73: tested only) and about 360 B live (the standing body's drive reset in #49, the golem link smoothing of #73, and the init's foot-mask union, which was live but not credited).
The sampler lane added about 3,070 B reproduced, tested only: the pose-only sampler's key pick, root source and height tail (#76, about 890 B), the living sampler's height tail and queued-clip switch (#81, about 880 B), the down body's probe (#49, about 150 B) and the placement routines' part, anchor, pin and mean arithmetic (#62 about 715 B, #58 about 440 B). Its one live change, the living hunch moved onto the thorax's own placement (#81), corrects code that was already credited, so the live total stays.
The muscles-off lane added about 900 B reproduced, tested only (the sampler is not scheduled): in #82 the bracket's branch order (the lead-in divides by the first key alone unless the blend is positive, no guard between the keys, a NaN clock between the keys), the progress (raw clock over the last key), the gait overlay by √run (from the capture's turn while it leads in) and the normalisation, the height target ((2·run + 2)·scale while walking, the −500 cm floor, the original's sum order) and the queued-clip switch (about 765 B); #84, the capture of the current targets, as `Animator::capture_targets` (about 140 B). Nothing live changed; #84's earlier live credit (about 70 B) was the living sampler's rebase, which is not this routine, so the live count drops to about 80,690 B (66 %).
The pain lane made #8's pain sounds live (about 530 B reproduced, about 1,050 B newly live with the tail already ported): after the damage call on flesh the wound set and the pain voice play, through each fighter's own voice (its sets by race template, voice number, sex and feral level; its line waits; volume × voice volume × sound gain; pitch × voice pitch). Lines 1 and 2 play the short and heavy groan sets and line 3 the grunt (the voice request's slots 1, 2 and 0, from the assembly); the earlier random human grunt on every blow (blocked and parried ones included) is gone.
The foot-sound lane made #18's footwork sounds live (about 400 B reproduced, about 900 B newly live with the rules already ported): the footwork's sound pass after the stance pick compares each foot's anchored bit with the state it last compared; a rise (a step started) plays the foley rustle at the standing place, a fall (a step ended) the race's footstep at the body root, each behind its own body clock (400 and 250 ms plus the generator's jitter), the draws, the play and the jitter in the original's order from one generator. The step length that sets the body's step loudness is now taken to the stance spot before the pull toward the other foot, as the original's.
The init's leg swap (#73) was checked against every shipped race model (all first thighs at negative x): nothing swaps, so the part-order legs are already the original's live; about 150 B credited live.
The feet-modes lane wired the down body's probe (#49, about 150 B, live: every body whose condition has the off-feet bit casts its 10 cm sphere from the pelvis before the feet, and a supported down body keeps that ground) and the feet routine's two modes (#50): the mode is picked before the per-foot pass, and both skip the probes, the support count, the level, the imbalance decay and the leg IK, give full support and aim the lift at the standing target (10 cm under it on the flat floor). About 290 B newly reproduced (the branches' skips, the lift and IK hand-off) and about 300 B newly live: the hovering mode runs for every body whose race has the hover flag (the wraith), now carried on the body's race factors. The flat-floor mode is wired to the state word's 0x1000 (Exarust's splay bit) but stays unreached: the original's lying spawn and its character viewers set that bit (assigned, never cleared), and Exarust's spawn does not set it. Placement on the ground (#62) is still not called: its callers are the AI's push out of a blocked region and four other paths, none of which Exarust has.
The spawn-order lane put the original's spawn behind the play flag `--original-spawn` (`exa_game limp.rs original_spawn_pose`): #76's key pick and root source, #58's placement (pose lowered by the pelvis's posed height onto the frame at that height) and #60's leg snap, its frame height then taken by the parts. Default play is unchanged, so the live totals stay; the flag's path calls about 2,000 B more of #58, #60 and #76. Checked against #60's assembly after a GPU run (player's pelvis snapped to 82.4 cm against the clip's 100.7, then lifted to about 95): the snap writes its height into the placement frame's translation and places each thigh at its offset from the root part on that frame, so the height is the pelvis's; its reach is a bent leg's (thigh and shin at the 0.766 cosine, minus the clip's horizontal ankle offset, shaped r² + (1 − r)√r over the leg length), so the original's placed pelvis also sits under the clip's and its lift raises it. The first spawn's key pick 1→0 is a looping clip's lead-in (from its last key to its first), as the default sample's. The snap's terms are logged ("placement snap: …") to check the reach against the original.
The spawnsnap lane (C13) made the original's spawn order the default and dropped `--original-spawn`. The level spawn places every living body at a matrix with the pose-only sampler's not-walking pelvis target (its condition word is still 0): the pelvis at the feet plus that target less 8 cm, the pose lowered onto it, no leg snap (the flag's snap at spawn, run twice, was Exarust's order and the source of its 4.5 cm). The player's behaviour on its third tick, after the move to the nearest walkable point (now asked at the pelvis's height), runs the placement on the ground (#62): the leg snap from the walkable floor with the body as it stands, its frame height taken by the frame, the current clip sampled and every part lowered onto it at rest, the feet re-anchored at their posed contacts less the clip root, the slack links restarted. The controller keeps its place: the original's frame sits at the pelvis and follows it, Exarust's capsule holds the feet. GPU: exanima01, arena and exanima06 players stand where they did (92.4 against 93.5, 95.3 and 95.3 cm over the capsule after 1 000 to 2 000 ticks), NPCs within a centimetre. About 1,000 B newly live (#58's arithmetic and target, #60, #62's sequence, #76's heights).
The charmove lane retired Exarust's capsule. Read in the assembly: the body tick's frame (#49) is a pure yaw at the character position with the pelvis's height, and nothing sweeps that position; its writers are the anchor routine (#20), the dead falling body and the get-up start (onto the pelvis), the placements (#58, #62), the spawn and the out-of-scene path follower; the per-tick grid snap on the player's and the AI's position is the grid's base slot, which returns false. Each fixed step the character now goes at once to its frame (the follow), at the character's ground the feet routine measured (#50's), with nothing swept; walls and props stop the body's parts. The feet routine's balance reset (standing, no support → 0) runs with no exception, as the original's (the perched rule is gone). GPU: the sewer's fixed tick fell from about 100 ms to under 10 ms (round 62's build beside this one in one grid); exanima01's stairs, the sewer and the arena walk with balance and support 1, walls and props stopping the body. About 60 B more of #49's tail (the frame at the character position) and #50's reset are now reproduced without a stand-in.

The reach-and-jolt lane added about 580 B reproduced and about 600 B newly live. In #50 the feet routine's per-leg pass measures the clip's hip-to-contact length squared each walking tick, and a get-up before 0.8 of its clip (and the flat floor) copies it into the lower leg's reach, which the leg relaxation then compares against for good (nothing restores the rest reach); the relaxation takes its rest and rise in double, takes a NaN target, and is skipped in the flat-floor and hovering modes as the original's per-leg pass is (about 215 B, live but for the flat floor). The flat-floor bit got a setter: the play flag `--flat-floor` gives the player the state word the original's lying spawn (its player spawn's mode 2, which its character viewers take) assigns. The placed spawn's own lying bit (its spawn description's word bit 1, distinct from the placement's condition) was not traced to a file field, so the level spawn does not set it; no shipped level places a living character with a condition other than 0. In #8 the player's feedback is live: a struck player's camera kicks by √(7Y + 7L) cm along the blade's velocity (with the kick routine's random offset) and its pad rumbles; a striking player's pad rumbles by half the yellow (about 250 B newly live). The swing's first clash or block (the routine's kind-2 branch, its swing flag 0x1000) now rings the striking weapon: its haft class's shared set (`WH_HaftWoodA`, `WH_HaftMA`, `WH_HaftLA`, read from the image) when the haft took the blow, else its own set, at volume × share and pitch × 1.2 (double), stamps its body's sound clock (now + 200 − round(80·volume)) and rumbles the player's pad by half the volume when the player is either side (about 260 B reproduced and live); a clash no longer plays the weapon's set on every contact. The struck item's own sound in that branch (#4) is still not called. In #49 the dead falling body's 5 cm probe before the fall-death call was read and is unreachable: a body whose condition is exactly 0xA00 at its tick's start returns before the posture block, and the word's three writers (the death routine, a save load and a corpse spawn) never run between that test and the probe's; credited as equivalent (about 100 B), nothing ported. The dying tick (#36) is done: its large-body branch writes the two "fatigue" factors, which nothing reads (about 58 B).

The motilepart lane (round 70) made #50's Ground Query live: the colliders built from a floor node's meshes (the second flag word's `0x800000`: 875 mesh nodes in 0.9.5, stairs, steps, bridges and floor grates among them, plus every tile grid cell's floor, which the original's grid loader merges into a `+Floor` node with the bit) answer `Level::floor_piece` (`exa_physics::FloorMesh`, `StaticColliders::is_floor`). The sole floor now takes the hit only when the low floor is under it (an unordered compare keeps the low floor). The query's rejection of a hit on a mesh's triangle 0 is not reproduced. The locomotion routine's grip-mode write (#31's run and walk modes) now reaches the held items (`exa_physics::grip::tick_mode`).

The footwork lane (round 66) put the player's combat footwork on the body's own step. Read in the assembly: the player behaviour's dispatcher and goal routine read and write the body's step fields (the stepping and balance-step bytes, the target, written in place by the goal routine, the kind, set to 2 by a tapped stride, the start clock, the queue word and vector), and its under-way test compares the target with the **feet centre**, not the chest. Exarust's player moved its character at a 1.4 m/s step velocity of its own and played the combat walk clip; now the step is the body's (`Ragdoll::footwork_step`, `write_footwork`), so the stance moves to the target, the feet step there by the stance score, the chest pins lead toward it, and the character follows the frame (the charmove gap); the stance clip plays throughout (the original never starts the combat walk). The body keeps the balance-step flag (set when the balance routine starts a step from rest; cleared by every other start, the step's end and any tick without a step) and the queue word (1 waiting, −2/−1 taken, 0 none; a knockback clears it); the queued-step taker (#17) takes a step only while the queue waits and leaves its countdown, the kind and balance flag cleared, its later stand-still stamp 0. The movement gate (#47), the lift servo's standing lead (#51), the leg routine's anchored-foot rule (#29), the arm setter's relaxed blend (#42) and the walks' look-ahead read the body's own episode (the lift servo and the look-ahead checked in the assembly), so a balance or knockback step now counts as well. The anchor routine (#20) now sets the standing point onto the pelvis every tick of a step (its stepping branch, read in the assembly; it was the routine's missing part), so the character follows a stepping body at once; and a tick without a step, or the step's end, clears the long kind as the feet routine and the step end do. The review's backward stumble was traced to three deviations in the body tick's pin block and the balance routine (read in the assembly): the root part's point record (Exarust's pelvis pin) is led while a step runs by its own rule (the error's cosine with the frame's forward axis, halved backwards, times its squared share of the step length forwards, 0 for a long step; × 0.7 or 0.4 in Idle, × (0.75·s + 1) × 1.2·scale × level), which Exarust lacked; both leads measure the step's offset from the standing point the anchor routine keeps (on the pelvis while stepping), where Exarust read a stale standing point, so its lead never faded; and the stance origin is clamped by the stored reach × 1.1 × scale (in double) for every kind, where Exarust took 1.5 × the reach for a long step (the length the feet routine rewrites has no reader there). The thorax reach narrows by the step episode, not by a stepping foot. GPU (arena, one grid): D, A, W and a three-second S hold walk on the feet with no fall (S 6.5 m back at balance 1). exanima01's spawn spot (the left foot's ground 13 cm higher) still trips on a held S: the stepping foot lifts 30 cm, its 300 ms stance bonus ends before it lands, and the other foot's start lands it in the air; not compared with retail. Traced further at that spot (second review): (a) the step lift's amplitude is set by the balance routine each tick the foot is chosen, 0.4 × the horizontal contact-to-anchor distance (at most 30 cm), and the feet routine lifts by −(1 − crouch)·f·(1.5 − the race's stance factor), f from the 3D distance; the anchor's height is the ground probed toward the anchor plus the sole (at most 1 m over the ankle): all as Exarust has them, except that Exarust read the body's stance factor (raised by the gait, zeroed by a knock-out) instead of the race's (fixed; equal, 0.5, during footwork). (b) The +2 stance bonus is a time window, 300 ms of the simulation clock from the foot's start, not scaled by the step and not held by any contact test. (c) The other foot's start lands an anchored foot where it is (anchor at its point, flags touching) with no gate but the choice. (d) The support test counts a foot whose contact height is at most its ground plus its offset plus 20 cm (30·scale moving) and that has ground in reach; the ground is the one probed toward the anchor; the step target has no ground term. At exanima01's spawn the held S's tap re-lays the step long 217 ms in (the dispatcher's rule), moving the stepping foot's anchor 45 cm farther while it is 30 cm up; its 300 ms bonus ends before it lands, the other foot is chosen, the stepping foot is landed 27 cm over its ground and the other one lifts, so no foot counts and the standing body's balance is set to 0. Each step of that chain is the original's rule; whether retail's foot travels as slowly there is not established (no retail capture with scripted keys). The placement on the ground (#62) now restarts the balance history as the original's tail does: the chest point on the placed parts as this tick's and last tick's, both chest velocities zero, every foot state cleared and the feet centre at the anchors' mean (NaN with no leg, as its 1/0); its pins and their mean are equivalent (taken anew before any reader). About 700 B newly reproduced and live (#62 about 310 B, #17 about 80 B, the balance flag, the gate and the readers about 300 B).

The backfall lane (round 73) traced the held-S falls at exanima01's spawn and in exanima06 (after a W into a wall and a D along it) to the step lift. Read in the assembly: the balance routine computes the chosen foot's lift scale (0.4 × its horizontal distance to the anchor, at most 30 cm) into a field of the **character**, while the feet routine reads the lift scale from the same offset of the **lower leg's own part**, which no instruction in the image writes and the class's creation leaves at zero; with a zero scale the feet routine's lift is −(1 − crouch)·0·(1.5 − stance) = 0 at any distance. The original's stepping foot is therefore never raised by a step lift (its standing IK reach gets nothing added); Exarust raised it up to 30 cm, so a long step on uneven or stepped ground was still in the air when its 300 ms stance bonus ended, the other foot's start landed it 27–31 cm over its ground, no foot counted and the balance went to 0. The earlier entry's (a) is corrected: the scale the feet routine reads is not the one the balance routine writes. Now `step_lift` takes the scale and the standing IK passes the zero the original reads. GPU (one grid, two runs each): exanima06's W/D/S script three runs with no fall (min balance 0.73, pelvis ≥ 90 cm, 2.2 m back on S); exanima01 spawn S hold twice with no fall (7 m back, balance 1); exanima01 stairs twice (one run a 20-tick Rebalance at the stair edge, where the standing foot's probe drops off the step, recovered; the other none); the arena walk with no fall. Separately, two runs of one scripted scene diverge from their first traced tick: the static collider build hands colliders over within a 4 ms wall-clock budget per frame, so when level geometry joins depends on the machine's speed, not on the frame count (step timing itself is per fixed tick and does not depend on the frame rate).

The AI-feet lane (round 86) put the AI's combat footwork on the body's own step. Read in the assembly: the AI's combat movement routine writes its chosen place into the body's step fields, the same ones the player's behaviour and routine #17 write (a new step: the target at the chest plus the place's offset, the stepping byte on, the balance-step byte off, the landing count and later stand-still stamp 0, the start clock now; a long step's second half: the kind 2 and the re-aimed, line-walked target with no new start; a step while one runs: the queue word 1 and the queued vector; a place of no step clears the queue word, a re-chosen step the kind; a queue word under 0 is turned over and cleared). It does not go through the player's word dispatcher. Exarust's duellist glided its character at a walk velocity in a "Combat Walk" clip, ending a step 10 cm from its goal or after 1 s; a duellist with muscles now reads and writes its body's step (`exa_ai lib.rs feet_outcome, apply_feet_steps`), keeps its stance and has no walk velocity, and its body's balance and feet routines walk the step. Routine #17's bend at the target's circle is applied to the AI's taken queue (the AI's `keep_off`, the approach clamp it already ports). GPU grid: the arena duel twice identical over 12,679 ticks; the opponent steps 15 times on its feet with no fall and reaches the scripted player (7 hits against 1 with the glide); exanima01 and exanima06 traces identical to the previous binary for every body (628 and 1,653 ticks).

The AI-walk lane (round 88) put the AI's route walking on the body's locomotion. Read in the assembly: the AI's locomotion driver, called by every walking state, writes the body's movement goal, heading, look and cursors, flags and moving byte (the fields the player's exploration steering writes) and returns busy while the body steps; the body tick's movement decision (routine #49's move and turn blocks) and locomotion routine (#31) then walk it, as they walk the player. Exarust's NPCs slid their characters at a capsule velocity in a walk or run clip with an instant turn; a fighter with muscles now carries the locomotion request (`exa_ai drive.rs`), its roam, patrol, flight, chase and follow drive it, and its body turns at the decision's rates (the idle states' slow turn) and walks on its feet, the walk clip started by #31 and ended in the stance when the body stops. Routine #17's bend now runs in the body's own balance pass along the look, for the player too. Routine #35 was checked: it is the statue body's out-of-sector mark, not the route slide; the route slide is the AI's path follower moving the standing point 2 cm a tick out of the scene, which needs the re-entry placement Exarust does not do yet. GPU grid (scripted, `--trace-all`): exanima01, 02 and 06 and the arena duel each run twice identical; against the previous binary exanima02's two falling gliders no longer fall (down ticks 3,319 to 0, balance under 0.6 2,437 to 0), and 2, 5 and 4 NPCs walk on their feet in exanima01, 02 and 06. Rechecked after the grip fix (weapons held in the hand again), the request on and off (`EXA_NPC_NO_LOCOMOTION`): the arena opponent lands 11 blows on the passive player over four scripts against 1 without it and stands as near or nearer (mean 93–129 cm against 99–119 cm); each scene repeats tick for tick; exanima01, 02 and 06 have no fall either way (exanima02's one down body is the same scripted get-up in both).

## Rebuilt against the code (round 127)

The component's rating still carried the "Missing" list of an early review (round 18). The table
was rebuilt routine by routine against the current crates; each item of that list is now:

| rating's missing item | routine | state now | where |
|---|---|---|---|
| gait audio mode | #31 | done, live: it is the grip mode the locomotion routine writes (run or in combat 1, else 2) | `exa_physics grip.rs tick_mode`, `body_gait.rs` |
| exertion and head tilt | #81, #31 | done, live: exertion kept by the locomotion routine, the head tilt its sign; the twist's own tilt field has no writer | `exa_physics body_gait.rs` |
| the dead jump branch | #49, #31 | dead in 0.9.5: its flag and timer have no writer | `Locomotor::jump` (kept, unreached) |
| the start's callback and part flag | #31, #92 | part flag live (`start_notice` clears the held items' motion flag); the callback is registered only by script and scene routines (twenty writers outside the class) and has no Exarust counterpart | `exa_physics body_gait.rs` |
| the Unkeyed start's clock | #31 | open (inferred 0) | - |
| held items' static-only collision flag | #49 | done, live | `exa_physics body.rs HELD_STATIC_ONLY`, `exa_game stages/movement.rs` |
| golem-suit halving and the anchored stepping foot in the leg routine | #29 | done, live | `exa_physics body_muscle.rs leg_gains, tune_muscles` |
| look target and timer | #49 | done, live (the look follow's rate 0.0125, rising 0.0025 to 0.15) | `exa_physics body_timers.rs`, `motile.rs follow_gain` |
| the head's anchor frame for clips >= 0x2C | #77, #81 | done, live (the head looks only in the first 0x2C clips and while not helpless, its weight reset otherwise; its anchors on the physical thorax) | `exa_game stages/sampler.rs LOOKING_CLIPS`, `exa_physics body_twist.rs head_look` |
| rsqrtss approximate normalisations | #14, #15, #31 | open: Exarust normalises exactly (the AI driver's rsqrt is ported) | - |
| the rest of the kernel, the solver, SetState and torque | #49 | the kernel at 72 % (table below); the solver and SetState belong to TDynamics and TMotilePart | - |
| femur posture, IK, foot pitch, gait plant events | #46, #14, #15, #50, #37 | done, live | `exa_physics body.rs femur_posture_turn`, `body_bend.rs`, `body_feet.rs` |

Ported in this round: the arm setter's tail (#42). Its sixteen calls in the body tick pass a held
factor, 2.0 everywhere but the attack's branch and the parry's (1.0; the relaxed blend's call is
the attack-arm easing routine's tail call, also 2.0); a shock and a kick make no call. After the
two arm chains it puts each held body's contact mobility (the pair stage's share) at its base
times that factor, the held list only when the holder is armed (class word & 0x30), the shield
always. The base is written once at the equip: a weapon's by its handling set-up (the mass-table
contact mobility over the holder's scale), a shield's from its mass value or, when that is 0, its
contact mobility. Exarust's base is the item body's registered mobility (the weapon-mass table is
not in the solver: a TMotilePart gap). GPU grid (round 127, `context/artifacts/2026-10-30-motiledyn`):
the factor is live (a guard's weapon 9.77 → 19.55 and back in each swing, the opponent's shield
0.55 → 1.10 and back to its base while its holder attacks or parries), and the arena duel, a shield duel and exanima01 are
tick-identical to main: the scripted contacts are the striker's weapon (factor 1, unchanged) and
held contacts the pair stage refuses.

## Largest remaining work

| routine | size | left (est.) | role | missing |
|---|---|---|---|---|
| #49 | 20,293 | 5,635 | Body tick kernel: prologue, impacts and burn, ramps, daze, links, behaviour, posture, attack, move/stop, turn, locomotion, twist and head, sampler, frame, carry, balance and feet calls | some state flags; the sampler call's tail and carry propagation |
| #8 | 13,157 | 1,845 | Damage routine: impact speed, zones, armour roll, channels, daze, block/clash/parry branches, bite, follow-through, sounds, blood, knockback, kill | bite link not attached; script hook and callbacks; the struck item's own sound (#4) in the clash branch not called |
| #18 | 8,061 | 1,209 | Balance routine: lean, score and counter, recovery steps and episodes, footwork and stance pick, grounds, support, landing and footstep sounds | wet step (no water surface); nav calls; parity unverified block by block |
| #76 | 3,562 | 1,250 | Pose-only sampler (spawn and placement) | the per-part routing (femur posture, half twist, head on the untwisted chain, hunch on the thorax alone) and the anchors as one pass; the walking target and the leg reach ported but not called |
| #82 | 4,499 | 585 | Muscles-off sampler | every part's contacts in the height (legs only); not scheduled |
| #50 | 7,185 | 835 | Feet routine: friction, height probe, smoothed ground, sole floor, anchor smoothing, pelvis target, support, lift servo and leg IK calls | the flat-floor bit only set by `--flat-floor`; the original's pelvis target (the carry's stands in); the leg relaxation runs in the muscle setter, gated on support, not per leg in reach; some thresholds unverified |
| #73 | 5,423 | 1,247 | Character init: rates, masses, gains, leg sides, placement, techniques, race and class presets, sounds, voice pitch, leg reach, head set-up | leg swap ported but not called; ghoul head-part set-up; gait scale check |
| #62 | 2,042 | 410 | Placement on the ground (teleport with snap) | its start (the probe set-up, the frame's ground), the extra part points, attached parts, cloth and attached bodies, behaviour hook; called only by the player's third behaviour tick |
| #58 | 2,194 | 880 | Placement at a matrix (spawn, teleport, saves, arena) | re-placing a living body, field resets, cloth, behaviour hook; anchors and pins ported but not called |
| #81 | 8,782 | 878 | Living sampler (muscles on) | exertion reads 0, dead jump branch, the queued-clip switch and the height tail ported but not called |

## Map

| # | bytes | role | Rust | status | missing | live |
|---|---|---|---|---|---|---|
| 1 | 355 | Class set-up: per-tick constants from the 83 Hz rate and the motile environment (default retentions × 0.6) | `exa_physics body.rs DRIVE_SCALE, MOTILE_ENVIRONMENT` | partial (45 %) | the other per-tick constants (1200/rate, 4π/3/rate, 80/rate, rate·0.0125); the environment is data no body uses | yes |
| 2 | 779 | Placement probe: 10 cm sphere cast down through collidable static meshes, skipping the character's own bodies | (stand-in: exa_physics contact.rs height_below) | missing | the mesh and body sweep with the own-body filter | no |
| 3 | 536 | Post-step: queued weapon impact into the damage routine, travel phase, looping voices follow the body, position history shift | `exa_physics body.rs Ragdoll::post_step, travel_phase` | partial (60 %) | voice follow and volume, history shift (damage arrives by message instead) | yes |
| 4 | 503 | Struck-item hit sound: cooldown, class set or item sound by where the point falls along the item, pitch ×1.2 | `exa_combat sounds.rs struck_item_sound` | done | NaN-projection branch picks the other set; no caller: its call is the clash branch of #8 (after the striking weapon's ring, now live), and Exarust has no struck item record split or length there; its clock is `ring_clock` and its three shared sets `HAFT_SETS` | no |
| 5 | 481 | Apply damage to the yellow and red bars, lasting-wound rule, low and kill thresholds, death | `exa_combat fighter.rs Health::take, failing` | done | player-kill callback hook | yes |
| 6 | 89 | Bite layers on the struck part, its parent and children | `exa_combat bite.rs stuck_layers` | done | - | no |
| 7 | 266 | Attacker recoil after a blocked or shield hit | `exa_combat swing.rs recoil, SwingState::recoil_from` | done | - | yes |
| 8 | 13,157 | Damage routine: impact speed, zones, armour roll, channels, daze, block/clash/parry branches, bite, follow-through, sounds, blood, knockback, kill | `exa_combat damage.rs resolve, hit_sounds, block, notices_damage`; `pain.rs pain_sounds, PainSounded, wound_set, struck_player_jolt, striking_player_rumble`; `voice.rs CharacterVoice`; `lib.rs react_to_hits, strike_back`; `exa_audio hits.rs pain_sounds`; `exa_game character_voice.rs`, `player.rs feel_blows`; `sounds.rs weapon_ring, ring_clock, HAFT_SETS`; `pain.rs BlowFelt, ClashFelt, clash_rumble` | partial (86 %) | bite link not attached; the scripted damage hook and the two damage callbacks; the struck item's own sound (#4) in the clash branch | yes |
| 9 | 651 | Bite: the weapon sticks in the struck part (grab link, strength, lifetime, weapon layer) | `exa_combat bite.rs stick, grab_of, BiteHold` | done | not wired into the hit path: the grab link joins two parts of one body, but the weapon is a separate entity (bone-attached or a gripped item body) and no cross-body grab exists | no |
| 10 | 490 | Follow-through after a wound | `exa_combat swing.rs follow_through, SwingState::after_wound` | done | - | yes |
| 11 | 286 | Push at a point, cut by how far the point already moves | `exa_physics body.rs push_fraction, Ragdoll::push_at` | done | no caller | no |
| 12 | 77 | Clear bits from every leg's foot-contact state | `exa_physics body.rs clear_feet`; `body_feet.rs clear_foot_state` | done | - | yes |
| 13 | 121 | Collision layer of the lower legs and feet | `exa_physics body_pairs.rs set_leg_layers`; `exa_combat condition.rs leg_layer_for` | done | - | yes |
| 14 | 2,251 | Standing leg IK (knee and hip by the law of cosines, clamps, easing, step lift) | `exa_physics body_bend.rs standing_leg, knee_angle, hip_angle, step_lift` | done | exact instead of approximate normalisation | yes |
| 15 | 1,814 | Locomoting leg IK (hip to anchor plus lag, speed-scaled easing) | `exa_physics body_bend.rs locomoting_leg, clip_hip_angle, target_hip_angle` | done | as routine #14 | yes |
| 16 | 610 | Step end: plant the best-placed free foot and move the standing point onto it | `exa_physics body_balance.rs end_step`; `body_steps.rs end_query` | partial (75 %) | zero-leg branch, nav-mesh clamp, timers | yes |
| 17 | 261 | Queued-step taker: target from the chest plus the queued step (bent at the target's circle), grid clamp and snap, stepping on, balance flag and landing count cleared, start clock, later stand-still stamp 0 | `exa_physics body_feet.rs step_end, queued_step, approach_bend`; `body_steps.rs take_queued`; `body_balance.rs measure_balance` (the take, the bend along the look at the body's target, `Ragdoll::aim_steps`, `look_steps`); `exa_game steering.rs Steering::queued_target`, `exa_ai lib.rs duellist` (the clamp) | partial (95 %) | the line walk and snap run on the behaviour's next tick from the chest then (the body has no grid) | yes (the player's and the AI's queue) |
| 18 | 8,061 | Balance routine: lean, score and counter, recovery steps and episodes, footwork and stance pick, grounds, support, landing and footstep sounds | `exa_physics body_balance.rs measure_balance`; `body_steps.rs`; `body_feet.rs foot_cues, FootCue, FootSounded, anchor_rustle, release_step, rustle_clock, release_step_clock, wet_release_step`; `exa_audio footsteps.rs foot_sounds` | partial (85 %) | the wet step (no level water surface in Exarust); the footstep plays one-shot, not on the two alternating held voices; the walk step's write of the shared step clock; nav calls; parity unverified block by block | yes |
| 19 | 945 | Stance score of a candidate foot placement | `exa_physics body_steps.rs stance_score, score_reference, foot_score, pick` | done | - | yes |
| 20 | 389 | Anchor routine: standing point follows the pelvis past a tolerance (set directly while stepping) | `exa_physics body_frame.rs follow, anchored, Ragdoll::anchor_frame` | partial (95 %) | its 1.5 × step-length clamp is unreachable in the original (guarded by the opposite test) and is not needed | yes |
| 21 | 85 | Full tone reset (stance factor kept, tone and stagger speed 1, decay 0, the get-up condition for motions 0x0E/0x0F) | `exa_physics body_feet.rs full_tone` | done | no caller: its one caller is the body tick's upkeep, every tick the muscles-on byte is 0 (not a get-up hook, as earlier written); whether a 0.9.5 body reaches that tick with its muscles off was not traced | no |
| 22 | 438 | Turn the heading toward a fall | `exa_game balance.rs fall_heading, turn_toward_fall` | done | - | yes |
| 23 | 614 | Getting up: pick the rise clip by the pelvis axis, turn, anchor, clear feet, start | `exa_game balance.rs start_getting_up, pick_rise, rise_heading` | done | dead-body frame branch | yes |
| 24 | 255 | Parry end and clinch clear (grip links off, hands' bit off, stance blend 40) | `exa_combat strike_body.rs clear_clinch`; `parry.rs ParryState::end` | done | clear_clinch has no caller: its links are the armed parry's grip links and the hands' bit 8, which Exarust never sets (the armed parry is not live), so the live end has nothing more to clear | partly |
| 25 | 1,881 | Armed parry: parry point on the attacker's blade, falloff, weapon or shield grip, passed-blow path | `exa_combat parry.rs parry_blade_height, blade_point, parry_point, parry_distance, armed_grip` | partial (80 %) | writing the grip links and the refusal path into the game (the armed parry is not modelled live) | no |
| 26 | 136 | Re-pick the parry clip for a new countered motion | `exa_combat parry.rs parry_repick` | done | - | yes |
| 27 | 788 | Auto-parry and clinch check each tick | `exa_combat parry.rs parry_ends, parry_trigger, parry_counter, parry_start, run_parries` | partial (70 %) | armed success, arm bit 8, explicit target | yes |
| 28 | 264 | Chest point (mean of the thorax pins) | `exa_physics body_balance.rs Ragdoll::chest_point`; `exa_combat stance.rs chest_point` | done | - | yes |
| 29 | 469 | Leg routine: thigh factor and leg-limit strength by leg mode | `exa_physics body_muscle.rs leg_gains, tune_muscles` | done | one child write unseen | yes |
| 30 | 67 | Can-act gate | `exa_ai combat.rs can_act`; `exa_physics motile.rs action_allowed` | done | - | yes |
| 31 | 3,389 | Locomotion routine: run blend, gait columns, boost ramp, speed, clock | `exa_physics body_gait.rs locomote, clock_step, advance_clock, stand`; `exa_physics grip.rs tick_mode` (its mode write) | partial (85 %) | the start callback (registered by the script and scene routines, not by the body; Exarust's start notice stands in); the Unkeyed start's clock; approximate normalisation. The "gait audio mode" of earlier ratings is the grip mode, live since round 70; the jump branch is dead (its flag has no writer) | yes |
| 32 | 1,375 | Stab arm drive: blade lift and bend push the weapon and forearm | `exa_combat weapon.rs blade_lift, blade_bend, handling_pushes` | done | not applied to any body | no |
| 33 | 36 | Smoothstep | `exa_combat swing.rs smoothstep` | done | - | yes |
| 34 | 6,380 | Swing tick: strike window, striker speeds, swing clock and stall, arm stiffness, sounds, weapon and hand pushes | `exa_combat swing.rs run_swings, pace_swings, attack_arm_stiffness, swing_sound`; `swing_voice.rs SwingVoice, SwingVoiceCue`; `exa_audio sounds.rs swing_voices`; `strike_body.rs fist_push, edge_push, overhead_pull, vertical_homing` | partial (80 %) | the pushes are not applied to bodies (tested only: most weapons are bone-attached, and a push on a gripped item body or a hand part is unverified against Exarust's solver); the armed-parry and clinch-clear branch at its top; down-attack arm flags; off-target countdown | yes (voice and rustle since round 29; pushes no) |
| 35 | 86 | Mark a statue body's scene-node objects (flag 4, recursively) when its node lies in no sector | - | missing | whole routine: the body tick calls it only for a statue body (the slow-turn word's bits 2 and 4, set by the sentinel set-up) whose node's sector mask is 0; Exarust keeps no per-node sector mask, so its trigger has no counterpart (not the out-of-scene route slide, which is the AI's path follower's) | n.a. |
| 36 | 578 | Collapse and dying tick | `exa_combat collapse.rs dying_tick, jitter_heading, lose_tone, fade_pins, stagger_step` | done | - (the large-body branch's two writes, 0.91 and 0.9 into the two "fatigue" factors, are dead stores: no reader in the image) | yes |
| 37 | 320 | Trailing foot planted on leaving Walk or Run | `exa_physics body_feet.rs plant_trailing_foot` | done | - | yes |
| 38 | 370 | Break Tangle: a hand part behind the back (in the character frame) builds a counter that starts the break motion | `exa_combat strike_body.rs break_tangle, break_tangles, hands_in_frame, TangleCount` | done | the hand's timer and the motion-state word it stamps (nothing in Exarust reads them) | yes |
| 39 | 801 | Burning damage over time | `exa_combat burn.rs burn_upkeep` | done | nothing feeds burn strength | yes |
| 40 | 90 | Group-root post-step: edge lengths, muscle-anchor history | `exa_physics body.rs edge restore`; `body_muscle.rs remember_anchors` | partial (80 %) | per-link hook unidentified | yes |
| 41 | 280 | Attack-arm easing | `exa_physics body_muscle.rs attack_arms` | done | - | yes |
| 42 | 515 | Arm setter: stiffness floor, relax cap, saturation, held items' contact mobility | `exa_physics body_muscle.rs arm_stiffness_floor, arm_relax_cap, arm_saturation, HELD_CONTACT, HELD_CONTACT_STRIKING` (`Drive::held_contact`); `exa_character grip_runtime.rs scale_held_contacts, held_contact_mobility` | done | the held items' base is their registered mobility, standing in for the weapon handling set-up's mass-table base (a TMotilePart gap); the chains' unread field (30) not kept | yes |
| 43 | 105 | Pit death sound | `exa_game fall.rs` | done | - | yes |
| 44 | 120 | Kill-plane death callback | `exa_combat fall.rs`; `exa_game fall.rs` | done | - | yes |
| 45 | 909 | Fall death: scream, 25 m loss, fade and death timers | `exa_combat fall.rs FallDeath`; `exa_game fall.rs fall_to_death` | done | - | yes |
| 46 | 472 | Femur posture (thigh turn about the femur axis) | `exa_physics body.rs femur_posture_turn`; `exa_game stages/sampler.rs femur_posture` | done | - | yes |
| 47 | 147 | Can-locomote gate | `exa_physics body_locomotion.rs gates`; `motile.rs action_ready` | done | - | yes |
| 48 | 41 | String release thunk (runtime library cleanup) | - | n.a. | - | n.a. |
| 49 | 20,293 | Body tick kernel: prologue, impacts and burn, ramps, daze, links, behaviour, posture, attack, move/stop, turn, locomotion, twist and head, sampler, frame, carry, balance and feet calls | `exa_physics body_stages.rs, grip.rs HeldGrip::relax, flip_step, body_gait.rs stand, body_feet.rs down_probe_origin, down_probe_ground`; `exa_character grip_runtime.rs`; `exa_game stages/*, balance.rs`; `exa_combat regen (ShieldStamina), condition, swing, parry, collapse` | partial (72 %) | some state flags (the dead falling body's 5 cm probe before the fall-death call is unreachable: credited as equivalent) | yes |
| 50 | 7,185 | Feet routine: friction, height probe, smoothed ground, sole floor, anchor smoothing, pelvis target, support, lift servo and leg IK calls | `exa_physics body_balance.rs measure_balance`; `body_feet.rs` (and `feet_mode, flat_floor_leg, flat_floor_target, HoverProbe`, run by `measure_balance`; `Drive::feet_mode` read by the lift and `bend_legs`); `body_lift.rs`; `body_bend.rs`; `body_steps.rs`; `body_muscle.rs relax_legs, reach_relax_target, relax_leg` (the lower leg's reach `Leg::reach2`) | partial (88 %) | the flat-floor bit is set only by the play flag `--flat-floor` (the level spawn's lying bit is untraced); the leg relaxation runs in the muscle setter gated on support, not per leg in reach; the modes aim Exarust's carry target (over the floor at 0, or over the hover ground) where the original sets its sampler's standing height; some thresholds unverified | yes (flat floor by flag only) |
| 51 | 712 | Pelvis-height lift servo | `exa_physics body_lift.rs lift_unclamped, lift_factor, Ragdoll::lift` | done | - | yes |
| 52 | 46 | Create the shared motions object once | `exa_anim lib.rs Motions` | done | - | yes |
| 53 | 696 | Constructor field defaults | `exa_physics body.rs Ragdoll::new`; `body_muscle.rs BodyStats` | partial (40 %) | many defaults unchecked | yes |
| 54 | 594 | Destructor | (ownership and despawn) | n.a. | - | n.a. |
| 55 | 67 | Clone | - | n.a. | - | n.a. |
| 56 | 70 | Bind the clip table to the skeleton | `exa_anim lib.rs Motions, attach_body_parts` | done | - | yes |
| 57 | 314 | Place parts at their nodes recursively | `exa_physics body.rs Ragdoll::place_at` | partial (40 %) | recursion into attached child bodies | yes |
| 58 | 2,194 | Placement at a matrix (spawn, teleport, saves, arena) | `exa_game limp.rs spawn_pose`; `exa_physics Ragdoll::place_at, placement_target, Locomotor::placed, clear_leg_corrections`; `body_placement.rs placed_parts, SPAWN_PELVIS_DROP, matrix_foot_anchor, matrix_pin, mean_point` | partial (60 %) | re-placing a living body, field resets, cloth, behaviour hook; the matrix anchors and pins are not called | spawn only (pose lowered onto the feet plus the target less 8 cm) |
| 59 | 1,549 | Init-time placement from the model nodes | (as routine #58) | partial (35 %) | field resets, contact offsets, the full capture step | yes |
| 60 | 1,515 | Placement leg snap (probe, mean ground, reach pick, anchors) | `exa_physics body_balance.rs snap_placement, leg_reach_cm, placement_feet`; `body_bend.rs snap_legs` | partial (88 %) | second query mode (the mesh and body sweep); the shin heights | yes (the player's third behaviour tick; the AI's re-entry, `exa_game limp.rs place_entering`; the frame now starts afresh: root aim, anchor and frame cleared) |
| 61 | 513 | Body reset when brought back into the scene | `exa_ai presence.rs` (the clip snapped to the stance, no capture); `exa_physics body.rs reset_presence` | partial (65 %) | part bit, callback, counters | yes |
| 62 | 2,042 | Placement on the ground (teleport with snap) | `exa_physics body_balance.rs Ragdoll::place_on_ground, Balance::placed` (snap in routine #60); `body_placement.rs placed_parts, ground_foot_anchor, mean_point`; `exa_game limp.rs GroundPlacement, place_on_ground` | partial (80 %) | its start (the probe set-up, the frame's ground and the standing points), the extra part points and attached parts, cloth and attached bodies, behaviour hook; the other callers (teleports, summons, saves, the AI's push out of a blocked region, the session's end). The pins and their mean are equivalent (the body tick takes both anew from the frame before any reader) | yes (player's third behaviour tick) |
| 63 | 648 | Head and limb set-up: mask backup, head gains, arm and hand factors, cloth | `exa_physics body.rs pose_share`; `body_muscle.rs HEAD_SHARE` | partial (40 %) | mask backup, arm ×1.25 and hand ×1.5, hand masks, cloth | yes |
| 64 | 33 | Set max health and fill both bars | `exa_combat fighter.rs Fighter::of_stats` | done | - | yes |
| 65 | 685 | Technique word from learnt skills | `exa_formats characters.rs CharacterRecord::perks, Perks::body_word` | done | held-item refresh call | yes |
| 66 | 181 | Scene list on hide and unhide | `exa_ai presence.rs show` | partial (50 %) | behaviour notification | yes |
| 67 | 26 | String release helper (runtime library) | - | n.a. | - | n.a. |
| 68 | 184 | Footstep sound set | `exa_audio voices.rs step_set`; `exa_game stages/movement.rs gait_footsteps` | done | wet steps not wired | yes |
| 69 | 140 | Voice set by race and sex | `exa_audio voices.rs voice_choice` | done | no caller | no |
| 70 | 412 | Two-hand group bit | `exa_combat strike_body.rs two_hand_group` | done | not wired: the body tick's prologue calls it every tick (mode 1 in the walk and run, else 0; mode 2/3 forced without a hands-bit class), on the group masks of two held items and their parent chain, which Exarust's held items (separate bodies, no shared group mask with the hands) do not carry | no |
| 71 | 168 | Add to the scene's dynamics list, player first | (spawn) | n.a. | - | n.a. |
| 72 | 360 | Part mobilities and masses | `exa_physics body.rs INVERSE_MASS, LINK_MOBILITY` | partial (40 %) | normalisation from measured size, limp parts | yes |
| 73 | 5,423 | Character init: rates, masses, gains, leg sides, placement, techniques, race and class presets, sounds, voice pitch, leg reach, head set-up | `exa_formats character_stats.rs CharacterStats::derive (RACE_PRESETS), BodyRules`; `exa_game scene_bodies.rs character_muscles`; `exa_combat sounds.rs`; `exa_physics body.rs Ragdoll::smooth_golem_links, legs_swap`; `body_pairs.rs register_legs` (foot-mask union) | partial (77 %) | the leg swap is not called: every shipped race model's first thigh sits at negative x (checked on all 17 exported race models, human −9.07 cm to the ogre's −20.2 cm), so part order is the original's leg order for every body the game builds (equivalent live; the mirrored test fixtures would swap); the ghouls' half-scale head-part set-up; a word set to 4 for creatures with no reader found; gait scale check | yes (leg swap equivalent) |
| 74 | 1,218 | Part classification: legs, side arms, hand chains | `exa_physics body.rs (fixed human bone order)` | partial (40 %) | data-driven walk, ankle offset, factors, roles | yes |
| 75 | 434 | Part list from model nodes | `exa_physics body.rs`; `exa_anim attach_body_parts`; `exa_combat sounds.rs` | partial (60 %) | boneless parts, per-part foley | yes |
| 76 | 3,562 | Pose-only sampler (spawn and placement) | `exa_game limp.rs spawn_pose, posed_parts`; `exa_anim sample, ClipPose::at, placement_bracket, placement_root_from_key`; `exa_physics body_pose_heights.rs stand_heights, placement_pelvis_target` (via `Ragdoll::placement_target`) | partial (65 %) | the per-part routing as one pass (femur posture, half twist, head on the untwisted chain, hunch on the thorax alone; at a first spawn all are identity) and the anchors; the walking target and the leg reach are not called | yes (key pick, stand height and the not-walking target at spawn) |
| 77 | 494 | Head look | `exa_physics body_twist.rs head_look`; `exa_game stages/sampler.rs` | partial (85 %) | head tilt, helpless decay | yes |
| 78 | 36 | Smooth ease | `exa_anim clock.rs Ease::apply` | done | - | yes |
| 79 | 60 | Fast ease | `exa_anim clock.rs Ease::apply` | done | - | yes |
| 80 | 668 | Gait floors | `exa_game limp.rs gait_floor`; `stages/movement.rs measure_ground` | done | one floor a tick instead of three | yes |
| 81 | 8,782 | Living sampler (muscles on) | `exa_game stages/sampler.rs`; `limp.rs clip_tick, run_overlay`; `exa_anim clock.rs` (and `switches_to_queued`); `exa_physics body_pose_heights.rs stand_heights, living_pelvis_target`; `body_gait.rs` (exertion, head tilt) | partial (90 %) | the height tail and the queued-clip switch are not called (Exarust's carry aims the pelvis from the feet; clips start at once). Exertion is kept by the locomotion routine (`body_gait.rs`, ×0.98 capped, ×0.99, ×0.95 standing) and its head tilt is the sign of it; the twist's tilt field has no writer (equivalent); the jump branch is dead | yes (heights and switch no) |
| 82 | 4,499 | Muscles-off sampler | `exa_game limp.rs sample_limp, limp_height_target, limp_foot`; `exa_anim clock.rs Animator::sample_ragdoll, ragdoll_bracket, ragdoll_overlay, RagdollSample` | partial (87 %) | the height reads the legs' contact points only (the original every part's contacts); the muscles-on root branch (unreachable behind the body tick's gate); the looping past-end share reads a register the routine never sets (taken as the clock); no caller, as in the original (the body tick skips it for every muscles-off body: traced for C8), and nothing queues a clip | equivalent (unreachable) |
| 83 | 553 | Full capture | `exa_anim clock.rs take_capture, capture_whole` | partial (70 %) | half lean on whole captures, femur-flag exception | yes |
| 84 | 230 | Capture the current targets (the samplers' switch to a queued clip) | `exa_anim clock.rs Animator::capture_targets` (called by `sample_ragdoll`'s switch) | partial (90 %) | the flags come from the feet only (the original's from every part with contacts); the living sampler's switch does not call it (it keeps no queued clip) | no |
| 85 | 280 | Physical hold pose | `exa_anim clock.rs Animator::hold_pose`; `limp.rs physical_turns` | done | - | yes |
| 86 | 380 | Daze pose | `exa_anim clock.rs shock_hold`; `exa_game stages/sampler.rs daze_pose`, `balance.rs ShockHeld` | done | - | yes |
| 87 | 564 | Plain capture | `exa_anim clock.rs take_capture`; `limp.rs Lean::capture`; `body_twist.rs half_lean` | done | explicit fallback to the full capture | yes |
| 88 | 85 | Set stance | `exa_combat stance.rs set_stance` | done | - | yes |
| 89 | 140 | Stance by energy | `exa_combat stance.rs stance_by_energy` (the AI's guard); `exa_combat parry.rs parry_stance` | partial (70 %) | recoil, failed-parry and clinch callers do not start the stance through it | yes |
| 90 | 92 | Cast end | `exa_game thaum/casting.rs end_cast`; `exa_combat effects.rs end_cast` | done | follow clip by Exarust's rule | yes |
| 91 | 157 | Cast start | `exa_game thaum/casting.rs start_cast`; `exa_combat effects.rs cast_start` | done | live start does not end the clinch | yes |
| 92 | 536 | Start a clip | `exa_anim clock.rs Animator::start, take_left_locomotion` | partial (55 %) | cast hook signal, clinch clear on leaving attacks, part-flag and list resets, start notice | yes |
| 93 | 432 | Render interpolation | `exa_game limp.rs`; `player_behaviour.rs` | partial (35 %) | health display, listener feed | yes |
| 94 | 58 | Render pre-step | `exa_physics controller.rs interpolate` | partial (40 %) | no dedicated port | yes |
| 95 | 243 | Translate the body and its points | `exa_physics body.rs Ragdoll::translate` | partial (60 %) | the character's own points | no |
| 96 | 104 | Add a power or status effect to the body | `exa_game thaum/cast.rs push_active` | done | - | yes |
| 97 | 27 | Focus spend | `exa_game thaum/casting.rs spend` | done | - | yes |
| 98 | 100 | Sustained focus tick | `exa_game thaum/casting.rs spend_tick` | done | - | yes |
| 99 | 764 | Focus settle and regeneration | `exa_game thaum/casting.rs settle_capacity, settle_focus` | done | timer flag clear | yes |
| 100 | 38 | Forward to a behaviour hook (role unknown) | - | missing | whole routine | n.a. |

## Body tick kernel (routine #49) by block

Byte counts are estimated from block landmarks in the research notes.

| block | bytes | share | notes |
|---|---|---|---|
| Prologue: fatigue factors, wound painting, dead hold, look smoothing, red regeneration | 1,030 | 70 % | body timers, red regeneration, blood |
| Impact accumulator, burn call, retention, grip helpers, readiness, stepping decay, queued motion, Break Tangle call | 980 | 50 % | grip relax, weapon flip and block stamina ported and live; Break Tangle called (round 29) |
| Pin gain and energy ramps, blow sag, daze and shock stagger | 760 | 80 % |  |
| Link-iteration target, jolt, grip relax and weapon flip, chest velocity, standing anchor, look target | 1,700 | 80 % | grip relax and flip: `exa_physics grip.rs`, run by `exa_character grip_runtime.rs` |
| Behaviour call, waking, collapse branch | 520 | 80 % |  |
| Posture block: falls, get-ups, weak, stance switch, clinch trigger | 4,370 | 80 % | not every state flag written |
| Attack and engage, muscles-off snap, focus tick, item effects, health bookkeeping | 1,860 | 65 % | block stamina: `exa_combat regen.rs ShieldStamina::tick`, live |
| Move and stop branches | 735 | 90 % |  |
| Movement decision | 490 | 80 % |  |
| Turn | 1,150 | 90 % |  |
| Alignment and stop | 600 | 95 % | drive cleared before the locomotion call (`stand`); the muscles-off clock counter has no writer (dead); the down body's ground probe runs in `measure_balance` before the feet (live) |
| Locomotion call and clock | 930 | 75 % |  |
| Torso twist and head look | 670 | 90 % |  |
| Sampler call, carry reset, jump phase, character frame, drive into the carry, part matrices, chest pins, balance and feet calls, carry propagation | 4,500 | 60 % | jump phase is dead code (its flag and timer have no writer; kept as `Locomotor::jump`); the character's place is the frame (no capsule since the charmove lane), kept at the feet's ground as Exarust's model root is at the feet |

## Corrections made by the audit

- The step routine an earlier rating credited as the step end is the step **start** (routine #17);
  the step end (routine #16) is `exa_physics body_balance.rs end_step`.
- Routine #20's clamp of the step target to 1.5 × step length sits behind a test that
  contradicts its enclosing test (the stepping flag clear, then set) and never runs.
- Break Tangle (routine #38) and the swing tick's weapon and hand pushes (routine #34) were
  thought missing; both are ported in `exa_combat strike_body.rs` but not called.
- The armed parry (routine #25): its blend was rounded half away from zero, the original's
  conversion rounds half to even; its grip strengths take their products in double; a NaN
  distance gives a NaN falloff. Fixed with the geometry port.
- The body tick's grip relax, weapon flip (routine #49 calls into the part class) and block
  stamina regeneration were listed missing; all three were already ported and live
  (`exa_physics grip.rs`, `exa_character grip_runtime.rs`, `exa_combat regen.rs`). Checked
  against the assembly: the flip and the stamina match; the relax steps of the second, arm and
  extra links are taken in double precision, fixed.
- The character init's race-byte branches (routine #73) mostly write fields the same init
  overwrites from the race record later; what survives is the balance bonus, the arm relaxation,
  the footstep volume (body scale, doubled over 1.3, the ogre's 5), the golem footstep set and
  golem suit, and the health and rule bits (already ported). Now `CharacterStats::derive`'s
  preset table; the spawn feeds the first two into the body's race muscles.
- Break Tangle (routine #38) reads the **hand parts**' origins (the character's hand list, the
  one whose entries' parents are the forearms), not the held items'; the box test is on the
  hand's place in the character frame. Its call (assembly) sits behind the condition word
  having no 0xA00 bit and the body's muscles-on byte, in the queued-motion block.
- The swing voice (routine #34) is the first held item's voice record, restarted (the
  environment's set retrigger, not a loop) silent at 0.4 × W26 on the region's first tick; its
  gain and pitch are written straight into that record on every attack tick, and the body
  tick's fade walks the held items' voices while the condition is not 2 (attacking). The
  character's swing loudness and sound gain are 1.0 from the constructor; only the force
  swing (loudness 3) and scripted fades change them.
- The body tick's alignment tail (routine #49), read from the assembly: after the alignment it
  zeroes the locomotion drive every tick before the locomotion call, so a body that does not
  locomote has no drive. Exarust kept the last walking tick's drive, which the carry still
  applied during a recovery step (`bearing.moving` is set while stepping); `stand` now clears it.
  A down body (condition 0x200) takes its ground from one 10 cm sphere cast 2000 cm down from
  the pelvis (not ported). The muscles-off branch's clock counter has no writer in the image:
  only a powered golem suit runs the clock there.
- The damage routine's tail (routine #8) after the damage call: the wound sound (`WndFls`,
  `SkelHit` or `HitGlm`, chosen at the init) and the pain voice (a groan above a wound of 0.3,
  else a groan or a grunt by the yellow over 0.5), then the "DamageTaken" notice (the earlier
  "camera nudge"; it is `notices_damage`), the two damage callbacks, and the player's feedback: a
  struck player's camera jolt and pad rumble by `7Y + 7L` (a big striker's by its speed), a
  striking player's rumble by half the yellow. A scripted damage hook, when set, replaces the
  damage call and skips all of it.
- The init's foot-mask union (routine #73) was already ported (`register_legs`). Its golem
  set-up gives every pivot link of every part the smoothing 0.995 (the earlier "contact
  smoothing"), now `Ragdoll::smooth_golem_links`, live for golems. Its leg order swaps the first
  two legs when the first leg's thigh sits at positive x in the root. The ghouls' head-part
  set-up re-runs the mass set-up of the character's attached head body at half its size factor.
- The feet routine's modes (routine #50): flat floor (the condition's 0x1000 bit) sets every
  leg's ground to 0, its anchor height to sole + 1, its state to 0x4a and its foot floor to 1,
  aims the pelvis 10 cm under its standing height and sets support and its factors to 1;
  hovering (the race's flag 0x200, the wraith) casts one 20 cm sphere 2000 cm down from 20 cm
  over the pelvis, plus 5 cm more after each miss.
- The down body's probe (routine #49) starts at the pelvis's position **plus the world
  translation of the zone its scene node hangs under** (the node's zone pointer, then that node's
  world position): the offset is the level sector's origin, as in the AI's place points, and is
  zero in Exarust's single world frame. The cast goes straight down (0, −1, 0); a hit's height is
  the body's ground, and the previous ground takes the ground whether it hit or not.
- The pose-only sampler (routine #76), walking condition, sets the pelvis target to (3 − 3·run) +
  (stand height + the pelvis's own posed height), not the living sampler's stand + 2·run + 3; off
  the walking condition both use stand (0 under Unconscious 1) + 2·scale + 5 when not stepping,
  × 0.7 or 0.8 (double) when crouched past the double 0.95 on the feet. Its bracket has no ease,
  no clamp and no guard between the keys, and its root comes from the clip's source key (not the
  capture) for the first three gaits with muscles on.
- The living sampler's hunch (routine #81) lowers the forward row of the pinned part's posed
  placement after the twist and leaves every other part's: Exarust pitched the thorax's local
  turn before the clip chain, so the head and arms followed it. Fixed (feral characters only). Its
  head tilt adds a field with no writer in the image, so the 0 Exarust reads is the original's.
- The placement routines (#58, #62) lower every part by the pelvis's posed height before putting
  it on the frame, so the pelvis lands at the frame's height; a foot anchor is the contact on the
  frame less the clip root, unrotated (the ground placement's height is the snapped ground plus
  the sole, the matrix placement's the mapped height plus the sole less 8 cm), and a matrix
  placement's pins sit 5 cm lower.
- The feet routine's flat-floor bit (the state word's 0x1000, earlier "written by no shipped
  code") is assigned by the original's lying spawn and by its character viewers (the whole word
  set to 0x1000, the femur posture called after); no routine clears it. It is the same bit as the
  femur posture's splay. The hovering mode's byte is set by the character init from the race
  record's flag 0x200 (and for race byte 14, the wraith). Neither mode calls the leg IK; both call
  the lift servo.
- The feet routine's copy of the clip's hip-to-contact length into the lower leg's reach (routine
  #50) is the flat-floor mode's only, not both modes': the hovering mode touches no leg. The copy
  is also made by the per-leg pass's getting-up branch (condition 0x300 before 0.8 of the clip,
  the ground in reach), and that one is observable: the leg relaxation reads the reach from then
  on. The relaxation itself (0.6 − 0.001·|d² − reach²|) takes its rest and its 0.01 rise in double.
- The struck-item sound (routine #4) is called from the damage routine's clash branch (its
  striker's contact kind 2), after the striking weapon's own ring, both once a swing (the striker's
  flag 0x1000); the earlier "shield-block branch" was wrong. The weapon's ring takes the haft
  class's shared set (`WH_HaftWoodA`, `WH_HaftMA`, `WH_HaftLA`) when the haft took the blow.
- The body tick's dead falling probe (routine #49: condition exactly 0xA00, its root particle's vertical
  step under −6 cm, a 5 cm sphere 2000 cm down, then the fall-death call) is unreachable:
  the same word returns the tick before the posture block, and nothing between the two tests
  writes it.
