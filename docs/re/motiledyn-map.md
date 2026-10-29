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
and leg order, the standing body's drive).

## Totals

| status | routines | bytes |
|---|---|---|
| done | 51 | 20,456 |
| partial | 41 | 101,464 |
| missing | 3 | 903 |
| n.a. | 5 | 896 |
| all | 100 | 123,719 |

Weighted by the partial shares, about **92,175 of 122,823 needed bytes (75 %)** are reproduced (n.a. routines left out; counting the partial routines' shares, 71,719 of their 101,464 bytes). Counting only what the running game calls, about **80,760 bytes (66 %)**; the rest is tested but not wired.
The wiring lane made Break Tangle (#38, 370 B), the swing voice of #34 (about 450 B) and the held-voice fade in #49 (about 100 B) live (about 80,400 B). The kernel lane then added about 1,670 B reproduced (the pain sounds and player jolt of #8, the feet modes of #50, the leg order of #73: tested only) and about 360 B live (the standing body's drive reset in #49, the golem link smoothing of #73, and the init's foot-mask union, which was live but not credited).

## Largest remaining work

| routine | size | left (est.) | role | missing |
|---|---|---|---|---|
| #49 | 20,293 | 5,885 | Body tick kernel: prologue, impacts and burn, ramps, daze, links, behaviour, posture, attack, move/stop, turn, locomotion, twist and head, sampler, frame, carry, balance and feet calls | the down body's ground probe, some state flags |
| #8 | 13,157 | 2,631 | Damage routine: impact speed, zones, armour roll, channels, daze, block/clash/parry branches, bite, follow-through, sounds, blood, knockback, kill | bite link not attached; pain sounds and player jolt ported but not wired; script hook and callbacks; struck-item sound not called |
| #76 | 3,562 | 2,315 | Pose-only sampler (spawn and placement) | femur posture, half twist, hunch, anchors, contacts, heights |
| #50 | 7,185 | 1,336 | Feet routine: friction, height probe, smoothed ground, sole floor, anchor smoothing, pelvis target, support, lift servo and leg IK calls | flat-floor and hovering modes ported but not wired; some thresholds unverified |
| #81 | 8,782 | 1,756 | Living sampler (muscles on) | head tilt and exertion read 0, dead jump branch, rebase capture |
| #62 | 2,042 | 1,736 | Placement on the ground (teleport with snap) | the outer sequence |
| #73 | 5,423 | 1,247 | Character init: rates, masses, gains, leg sides, placement, techniques, race and class presets, sounds, voice pitch, leg reach, head set-up | leg swap ported but not called; ghoul head-part set-up; gait scale check |
| #18 | 8,061 | 1,612 | Balance routine: lean, score and counter, recovery steps and episodes, footwork and stance pick, grounds, support, landing and footstep sounds | foot sounds ported but not wired; nav calls; parity unverified block by block |
| #58 | 2,194 | 1,426 | Placement at a matrix (spawn, teleport, saves, arena) | re-placing a living body, field resets, contact offsets, cloth, behaviour hook |
| #82 | 4,499 | 1,350 | Muscles-off sampler | hold, anchors; not scheduled |

## Map

| # | bytes | role | Rust | status | missing | live |
|---|---|---|---|---|---|---|
| 1 | 355 | Class set-up: per-tick constants from the 83 Hz rate and the motile environment (default retentions × 0.6) | `exa_physics body.rs DRIVE_SCALE, MOTILE_ENVIRONMENT` | partial (45 %) | the other per-tick constants (1200/rate, 4π/3/rate, 80/rate, rate·0.0125); the environment is data no body uses | yes |
| 2 | 779 | Placement probe: 10 cm sphere cast down through collidable static meshes, skipping the character's own bodies | (stand-in: exa_physics contact.rs height_below) | missing | the mesh and body sweep with the own-body filter | no |
| 3 | 536 | Post-step: queued weapon impact into the damage routine, travel phase, looping voices follow the body, position history shift | `exa_physics body.rs Ragdoll::post_step, travel_phase` | partial (60 %) | voice follow and volume, history shift (damage arrives by message instead) | yes |
| 4 | 503 | Struck-item hit sound: cooldown, class set or item sound by where the point falls along the item, pitch ×1.2 | `exa_combat sounds.rs struck_item_sound` | done | NaN-projection branch picks the other set; no caller: its call is the shield-block branch of #8, and Exarust's shield guard has no item entity, record split or length, nor the three shared class sets | no |
| 5 | 481 | Apply damage to the yellow and red bars, lasting-wound rule, low and kill thresholds, death | `exa_combat fighter.rs Health::take, failing` | done | player-kill callback hook | yes |
| 6 | 89 | Bite layers on the struck part, its parent and children | `exa_combat bite.rs stuck_layers` | done | - | no |
| 7 | 266 | Attacker recoil after a blocked or shield hit | `exa_combat swing.rs recoil, SwingState::recoil_from` | done | - | yes |
| 8 | 13,157 | Damage routine: impact speed, zones, armour roll, channels, daze, block/clash/parry branches, bite, follow-through, sounds, blood, knockback, kill | `exa_combat damage.rs resolve, hit_sounds, block, notices_damage`; `pain.rs pain_sounds, struck_player_jolt, striking_player_rumble`; `lib.rs react_to_hits, strike_back` | partial (80 %) | bite link not attached; the pain sounds and the player jolt are not called (no voice samples are played yet, no camera jolt or pad rumble reads them); the scripted damage hook and the two damage callbacks; struck-item sound not called | yes (pain sounds no) |
| 9 | 651 | Bite: the weapon sticks in the struck part (grab link, strength, lifetime, weapon layer) | `exa_combat bite.rs stick, grab_of, BiteHold` | done | not wired into the hit path: the grab link joins two parts of one body, but the weapon is a separate entity (bone-attached or a gripped item body) and no cross-body grab exists | no |
| 10 | 490 | Follow-through after a wound | `exa_combat swing.rs follow_through, SwingState::after_wound` | done | - | yes |
| 11 | 286 | Push at a point, cut by how far the point already moves | `exa_physics body.rs push_fraction, Ragdoll::push_at` | done | no caller | no |
| 12 | 77 | Clear bits from every leg's foot-contact state | `exa_physics body.rs clear_feet`; `body_feet.rs clear_foot_state` | done | - | yes |
| 13 | 121 | Collision layer of the lower legs and feet | `exa_physics body_pairs.rs set_leg_layers`; `exa_combat condition.rs leg_layer_for` | done | - | yes |
| 14 | 2,251 | Standing leg IK (knee and hip by the law of cosines, clamps, easing, step lift) | `exa_physics body_bend.rs standing_leg, knee_angle, hip_angle, step_lift` | done | exact instead of approximate normalisation | yes |
| 15 | 1,814 | Locomoting leg IK (hip to anchor plus lag, speed-scaled easing) | `exa_physics body_bend.rs locomoting_leg, clip_hip_angle, target_hip_angle` | done | as routine #14 | yes |
| 16 | 610 | Step end: plant the best-placed free foot and move the standing point onto it | `exa_physics body_balance.rs end_step`; `body_steps.rs end_query` | partial (75 %) | zero-leg branch, nav-mesh clamp, timers | yes |
| 17 | 261 | Step start: AI notice, target from the base plus the step, stepping on, timers | `exa_physics body_feet.rs step_end, queued_step` | partial (55 %) | nav clamp, timers; the AI notice is read by nothing | yes |
| 18 | 8,061 | Balance routine: lean, score and counter, recovery steps and episodes, footwork and stance pick, grounds, support, landing and footstep sounds | `exa_physics body_balance.rs measure_balance`; `body_steps.rs`; `body_feet.rs anchor_rustle, release_step, wet_release_step`; `exa_audio footsteps.rs` | partial (80 %) | the foot sounds are not called (Exarust's steps come from the sampler's plants); nav calls; parity unverified block by block | yes |
| 19 | 945 | Stance score of a candidate foot placement | `exa_physics body_steps.rs stance_score, score_reference, foot_score, pick` | done | - | yes |
| 20 | 389 | Anchor routine: standing point follows the pelvis past a tolerance (set directly while stepping) | `exa_physics body_frame.rs follow, anchored, Ragdoll::anchor_frame` | partial (80 %) | the direct set while stepping; its 1.5 × step-length clamp is unreachable in the original (guarded by the opposite test) and is not needed | yes |
| 21 | 85 | Full tone reset after a get-up | `exa_physics body_feet.rs full_tone` | done | no caller | no |
| 22 | 438 | Turn the heading toward a fall | `exa_game balance.rs fall_heading, turn_toward_fall` | done | - | yes |
| 23 | 614 | Getting up: pick the rise clip by the pelvis axis, turn, anchor, clear feet, start | `exa_game balance.rs start_getting_up, pick_rise, rise_heading` | done | dead-body frame branch | yes |
| 24 | 255 | Parry end and clinch clear (grip links off, hands' bit off, stance blend 40) | `exa_combat strike_body.rs clear_clinch`; `parry.rs ParryState::end` | done | clear_clinch has no caller: its links are the armed parry's grip links and the hands' bit 8, which Exarust never sets (the armed parry is not live), so the live end has nothing more to clear | partly |
| 25 | 1,881 | Armed parry: parry point on the attacker's blade, falloff, weapon or shield grip, passed-blow path | `exa_combat parry.rs parry_blade_height, blade_point, parry_point, parry_distance, armed_grip` | partial (80 %) | writing the grip links and the refusal path into the game (the armed parry is not modelled live) | no |
| 26 | 136 | Re-pick the parry clip for a new countered motion | `exa_combat parry.rs parry_repick` | done | - | yes |
| 27 | 788 | Auto-parry and clinch check each tick | `exa_combat parry.rs parry_ends, parry_trigger, parry_counter, parry_start, run_parries` | partial (70 %) | armed success, arm bit 8, explicit target | yes |
| 28 | 264 | Chest point (mean of the thorax pins) | `exa_physics body_balance.rs Ragdoll::chest_point`; `exa_combat stance.rs chest_point` | done | - | yes |
| 29 | 469 | Leg routine: thigh factor and leg-limit strength by leg mode | `exa_physics body_muscle.rs leg_gains, tune_muscles` | done | one child write unseen | yes |
| 30 | 67 | Can-act gate | `exa_ai combat.rs can_act`; `exa_physics motile.rs action_allowed` | done | - | yes |
| 31 | 3,389 | Locomotion routine: run blend, gait columns, boost ramp, speed, clock | `exa_physics body_gait.rs locomote, clock_step, advance_clock, stand` | partial (80 %) | gait audio mode, dead jump branch, start callback, approximate normalisation | yes |
| 32 | 1,375 | Stab arm drive: blade lift and bend push the weapon and forearm | `exa_combat weapon.rs blade_lift, blade_bend, handling_pushes` | done | not applied to any body | no |
| 33 | 36 | Smoothstep | `exa_combat swing.rs smoothstep` | done | - | yes |
| 34 | 6,380 | Swing tick: strike window, striker speeds, swing clock and stall, arm stiffness, sounds, weapon and hand pushes | `exa_combat swing.rs run_swings, pace_swings, attack_arm_stiffness, swing_sound`; `swing_voice.rs SwingVoice, SwingVoiceCue`; `exa_audio sounds.rs swing_voices`; `strike_body.rs fist_push, edge_push, overhead_pull, vertical_homing` | partial (80 %) | the pushes are not applied to bodies (tested only: most weapons are bone-attached, and a push on a gripped item body or a hand part is unverified against Exarust's solver); the armed-parry and clinch-clear branch at its top; down-attack arm flags; off-target countdown | yes (voice and rustle since round 29; pushes no) |
| 35 | 86 | Mark every scene node's object when the body leaves every sector | - | missing | whole routine | n.a. |
| 36 | 578 | Collapse and dying tick | `exa_combat collapse.rs dying_tick, jitter_heading, lose_tone, fade_pins, stagger_step` | partial (90 %) | large-body branch's two writes | yes |
| 37 | 320 | Trailing foot planted on leaving Walk or Run | `exa_physics body_feet.rs plant_trailing_foot` | done | - | yes |
| 38 | 370 | Break Tangle: a hand part behind the back (in the character frame) builds a counter that starts the break motion | `exa_combat strike_body.rs break_tangle, break_tangles, hands_in_frame, TangleCount` | done | the hand's timer and the motion-state word it stamps (nothing in Exarust reads them) | yes |
| 39 | 801 | Burning damage over time | `exa_combat burn.rs burn_upkeep` | done | nothing feeds burn strength | yes |
| 40 | 90 | Group-root post-step: edge lengths, muscle-anchor history | `exa_physics body.rs edge restore`; `body_muscle.rs remember_anchors` | partial (80 %) | per-link hook unidentified | yes |
| 41 | 280 | Attack-arm easing | `exa_physics body_muscle.rs attack_arms` | done | - | yes |
| 42 | 515 | Arm setter: stiffness floor, relax cap, saturation, held items' mobility | `exa_physics body_muscle.rs arm_stiffness_floor, arm_relax_cap, arm_saturation` | partial (75 %) | held items' mobility factor | yes |
| 43 | 105 | Pit death sound | `exa_game fall.rs` | done | - | yes |
| 44 | 120 | Kill-plane death callback | `exa_combat fall.rs`; `exa_game fall.rs` | done | - | yes |
| 45 | 909 | Fall death: scream, 25 m loss, fade and death timers | `exa_combat fall.rs FallDeath`; `exa_game fall.rs fall_to_death` | done | - | yes |
| 46 | 472 | Femur posture (thigh turn about the femur axis) | `exa_physics body.rs femur_posture_turn`; `exa_game stages/sampler.rs femur_posture` | done | - | yes |
| 47 | 147 | Can-locomote gate | `exa_physics body_locomotion.rs gates`; `motile.rs action_ready` | done | - | yes |
| 48 | 41 | String release thunk (runtime library cleanup) | - | n.a. | - | n.a. |
| 49 | 20,293 | Body tick kernel: prologue, impacts and burn, ramps, daze, links, behaviour, posture, attack, move/stop, turn, locomotion, twist and head, sampler, frame, carry, balance and feet calls | `exa_physics body_stages.rs, grip.rs HeldGrip::relax, flip_step, body_gait.rs stand`; `exa_character grip_runtime.rs`; `exa_game stages/*, balance.rs`; `exa_combat regen (ShieldStamina), condition, swing, parry, collapse` | partial (71 %) | the down body's ground probe under the pelvis; some state flags | yes |
| 50 | 7,185 | Feet routine: friction, height probe, smoothed ground, sole floor, anchor smoothing, pelvis target, support, lift servo and leg IK calls | `exa_physics body_balance.rs measure_balance`; `body_feet.rs` (and `feet_mode, flat_floor_leg, flat_floor_target, HoverProbe`); `body_lift.rs`; `body_bend.rs`; `body_steps.rs` | partial (81 %) | the flat-floor and hovering modes are not called (no Exarust body has the flat-floor bit; the wraith's hover flag is not passed to its body); some thresholds unverified | yes (modes no) |
| 51 | 712 | Pelvis-height lift servo | `exa_physics body_lift.rs lift_unclamped, lift_factor, Ragdoll::lift` | done | - | yes |
| 52 | 46 | Create the shared motions object once | `exa_anim lib.rs Motions` | done | - | yes |
| 53 | 696 | Constructor field defaults | `exa_physics body.rs Ragdoll::new`; `body_muscle.rs BodyStats` | partial (40 %) | many defaults unchecked | yes |
| 54 | 594 | Destructor | (ownership and despawn) | n.a. | - | n.a. |
| 55 | 67 | Clone | - | n.a. | - | n.a. |
| 56 | 70 | Bind the clip table to the skeleton | `exa_anim lib.rs Motions, attach_body_parts` | done | - | yes |
| 57 | 314 | Place parts at their nodes recursively | `exa_physics body.rs Ragdoll::place_at` | partial (40 %) | recursion into attached child bodies | yes |
| 58 | 2,194 | Placement at a matrix (spawn, teleport, saves, arena) | `exa_game limp.rs spawn_pose`; `exa_physics Ragdoll::place_at, Locomotor::placed, clear_leg_corrections` | partial (35 %) | re-placing a living body, field resets, contact offsets, cloth, behaviour hook | spawn only |
| 59 | 1,549 | Init-time placement from the model nodes | (as routine #58) | partial (35 %) | field resets, contact offsets, the full capture step | yes |
| 60 | 1,515 | Placement leg snap (probe, mean ground, reach pick, anchors) | `exa_physics body_balance.rs snap_placement, leg_reach_cm, placement_feet`; `body_bend.rs snap_legs` | partial (85 %) | second query mode, frame height write-back | no |
| 61 | 513 | Body reset when brought back into the scene | `exa_ai presence.rs`; `exa_physics body.rs reset_presence` | partial (50 %) | idle clip, part bit, callback, counters | yes |
| 62 | 2,042 | Placement on the ground (teleport with snap) | (snap in routine #60 only) | partial (15 %) | the outer sequence | no |
| 63 | 648 | Head and limb set-up: mask backup, head gains, arm and hand factors, cloth | `exa_physics body.rs pose_share`; `body_muscle.rs HEAD_SHARE` | partial (40 %) | mask backup, arm ×1.25 and hand ×1.5, hand masks, cloth | yes |
| 64 | 33 | Set max health and fill both bars | `exa_combat fighter.rs Fighter::of_stats` | done | - | yes |
| 65 | 685 | Technique word from learnt skills | `exa_formats characters.rs CharacterRecord::perks, Perks::body_word` | done | held-item refresh call | yes |
| 66 | 181 | Scene list on hide and unhide | `exa_ai presence.rs show` | partial (50 %) | behaviour notification | yes |
| 67 | 26 | String release helper (runtime library) | - | n.a. | - | n.a. |
| 68 | 184 | Footstep sound set | `exa_audio voices.rs step_set`; `exa_game stages/movement.rs gait_footsteps` | done | wet steps not wired | yes |
| 69 | 140 | Voice set by race and sex | `exa_audio voices.rs voice_choice` | done | no caller | no |
| 70 | 412 | Two-hand group bit | `exa_combat strike_body.rs two_hand_group` | done | no caller | no |
| 71 | 168 | Add to the scene's dynamics list, player first | (spawn) | n.a. | - | n.a. |
| 72 | 360 | Part mobilities and masses | `exa_physics body.rs INVERSE_MASS, LINK_MOBILITY` | partial (40 %) | normalisation from measured size, limp parts | yes |
| 73 | 5,423 | Character init: rates, masses, gains, leg sides, placement, techniques, race and class presets, sounds, voice pitch, leg reach, head set-up | `exa_formats character_stats.rs CharacterStats::derive (RACE_PRESETS), BodyRules`; `exa_game scene_bodies.rs character_muscles`; `exa_combat sounds.rs`; `exa_physics body.rs Ragdoll::smooth_golem_links, legs_swap`; `body_pairs.rs register_legs` (foot-mask union) | partial (77 %) | the leg swap is not called (the human's first thigh is at negative x, where nothing swaps); the ghouls' half-scale head-part set-up; a word set to 4 for creatures with no reader found; gait scale check | yes (leg swap no) |
| 74 | 1,218 | Part classification: legs, side arms, hand chains | `exa_physics body.rs (fixed human bone order)` | partial (40 %) | data-driven walk, ankle offset, factors, roles | yes |
| 75 | 434 | Part list from model nodes | `exa_physics body.rs`; `exa_anim attach_body_parts`; `exa_combat sounds.rs` | partial (60 %) | boneless parts, per-part foley | yes |
| 76 | 3,562 | Pose-only sampler (spawn and placement) | `exa_game limp.rs spawn_pose`; `exa_anim sample, ClipPose::at` | partial (35 %) | femur posture, half twist, hunch, anchors, contacts, heights | yes |
| 77 | 494 | Head look | `exa_physics body_twist.rs head_look`; `exa_game stages/sampler.rs` | partial (85 %) | head tilt, helpless decay | yes |
| 78 | 36 | Smooth ease | `exa_anim clock.rs Ease::apply` | done | - | yes |
| 79 | 60 | Fast ease | `exa_anim clock.rs Ease::apply` | done | - | yes |
| 80 | 668 | Gait floors | `exa_game limp.rs gait_floor`; `stages/movement.rs measure_ground` | done | one floor a tick instead of three | yes |
| 81 | 8,782 | Living sampler (muscles on) | `exa_game stages/sampler.rs`; `limp.rs clip_tick, run_overlay`; `exa_anim clock.rs` | partial (80 %) | head tilt and exertion read 0, dead jump branch, rebase capture | yes |
| 82 | 4,499 | Muscles-off sampler | `exa_game limp.rs sample_limp, limp_foot`; `exa_anim clock.rs sample_ragdoll` | partial (70 %) | hold, anchors; not scheduled | no |
| 83 | 553 | Full capture | `exa_anim clock.rs take_capture, capture_whole` | partial (70 %) | half lean on whole captures, femur-flag exception | yes |
| 84 | 230 | Capture current targets at a rebase | `exa_anim clock.rs Animator::bracket` | partial (30 %) | the rebase drops the capture instead of storing targets | yes |
| 85 | 280 | Physical hold pose | `exa_anim clock.rs Animator::hold_pose`; `limp.rs physical_turns` | done | - | yes |
| 86 | 380 | Daze pose | `exa_anim clock.rs shock_hold` | partial (80 %) | no caller; its capture source (#84) | no |
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
| Alignment and stop | 600 | 65 % | drive cleared before the locomotion call (`stand`); the muscles-off clock counter has no writer (dead); the down body's ground probe is not ported |
| Locomotion call and clock | 930 | 75 % |  |
| Torso twist and head look | 670 | 90 % |  |
| Sampler call, carry reset, jump phase, character frame, drive into the carry, part matrices, chest pins, balance and feet calls, carry propagation | 4,500 | 60 % | jump phase is dead code (its flag and timer have no writer; kept as `Locomotor::jump`); part of the movement still follows the capsule controller |

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
