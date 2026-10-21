## 1. Research

- [x] 1.1 Read the body tick's twist block and the living sampler's midline step in `exe.asm`; the helpers (matrix product, lerp, Gram-Schmidt, cross, normalisations, arc cosine, direction yaw, look matrix); the midline bit's builder; constants from the PE; notes in `context/re/locomotion-kernel.md`

## 2. Implementation

- [x] 2.1 `exa_physics::body::locomotion`: `acos_approx`, `heading_of`, `look_rotation`, `TwistBy`, `twist`, `twist_spine`; `Locomotor::twist_yaw`, `Locomotor::twist`
- [x] 2.2 `stages::movement`: the twist after locomotion from last tick's yaw, the look, the condition, the tone, the swing and the crouch
- [x] 2.3 `stages::sampler`: the midline chain (`spine`) and step 7

## 3. Tests

- [x] 3.1 `exa_physics`: the arc cosine (1.046816, 2.0947766, π/2, 0, π) and the yaw (0.785179, sign, vertical); the follow (0.05, 0.500297 after 11, 0.02, 0.072, the 1 rad clamp, the frame, held helpless and at 0.85); the direction ((0, −0.6, 0.8), its up (0, 0.8, 0.6), casting (0, −0.57735, 0.8165), attacking (0, −0.21951, 0.97561)); the spine (the lumbar half, the thorax whole, positions kept, the identity)
- [x] 3.2 `exa_game`: the midline chain (lumbar half, thorax, not the head or the thigh; 3 cm ends it); the stage's twist follows the look and holds down
- [x] 3.3 Real-data check: none needed (the midline is read from each body's own rest pose)

## 4. Docs and checks

- [x] 4.1 `docs/formats/RSQ.md` (Torso twist, the pipeline table), `docs/PLAYER.md`
- [x] 4.2 `cargo fmt --all`, remote `test` and `clippy -D warnings`; `openspec validate --all --strict`
