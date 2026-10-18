# Tasks

## 1. Pin pull

- [x] 1.1 Add the pure pin pull and the target clamp (thorax reach, pelvis reach, the 5·scale cm unsupported shift) in the body iteration, centimetres through the existing cm conversion, and verify one application matches the law: 10 cm with 1 cm of displacement gives −0.0333 cm, 80 cm with none gives 0.4 cm, getting up is 0.3 of the standing pull, and no support gives zero.
- [x] 1.2 Stop the integration step from pulling or braking the pelvis by 0.168 × tone² × energy⁴, and verify a standing pelvis offset across the ground is not moved by that spring.

## 2. Body behaviour

- [x] 2.1 Verify on a synthetic standing body, through the real fixed step, that a chest about 10 cm off the feet centre returns toward the feet over later ticks, that an unsupported target only shifts 5·scale cm forward, and that a moving body is not held on its starting world spot.
- [x] 2.2 Update `docs/formats/RSQ.md`, `docs/PLAN.md` (Phase 3 and §7) and `crates/exa_physics/README.md` so the pins are present behaviour and recovery steps, leg IK and gait walking stay not done. Verify those pages no longer list the support pins as future work.

## 3. Check

- [x] 3.1 Run `make check` and `openspec validate --all --strict`, and verify both pass. Corpus: no format change; the races' pin factor the pull reads is already pinned by the character-stats tests, skipped here when `EXANIMA_DIR` is unset.

## Workflow follow-up

- Archive `pull-the-support-pins` after the check passes, so `openspec/specs/physical-body` gains the pin requirements.
