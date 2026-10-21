## 1. Research

- [x] 1.1 Read the sampler's head step and the head look routine in `exe.asm`; the angle helpers, the arc sine polynomial; constants from the PE; the pitch offset's writers (none found); notes in `context/re/locomotion-kernel.md`

## 2. Implementation

- [x] 2.1 `exa_physics::body::locomotion`: `asin_approx`, `euler_of`, `from_euler`, `head_look`; `Locomotor::frame_yaw`, `head_weight`, `head_yaw`
- [x] 2.2 `Bearing::head_looks` and the head's anchors relative to the physical thorax (`body_muscle.rs`)
- [x] 2.3 `stages::sampler`: the look's input (`look_of`) and step 8 (`look_with_head`); `stages::movement` keeps the frame's yaw

## 3. Tests

- [x] 3.1 `exa_physics`: the arc sine (0.52398, odd, 0, π/2); the angles (yaw, pitch, roll alone; the round trip; the roll at π/2); the head look (π/100 a tick, 0.40024, half weight, pitch 0.52398, the held decay 0.036); the looking head's anchors follow the physical thorax
- [x] 3.2 `exa_game`: the look's input (0.3 rad in a frame at 0.2, the weight 0.01 to 1, none and 0 when not looking, the dazed head's own forward)
- [x] 3.3 Real-data check: none needed (the head is found by its bone)

## 4. Docs and checks

- [x] 4.1 `docs/formats/RSQ.md` (Head look, the pipeline table), `docs/PLAYER.md`
- [x] 4.2 `cargo fmt --all`, remote `test` and `clippy -D warnings`; `openspec validate --all --strict`
