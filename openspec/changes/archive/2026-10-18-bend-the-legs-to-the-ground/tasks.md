## 1. Read the two open fields

- [x] 1.1 With REA, read which side the 0.02 knee blend weights and the centimetre drop of an unanchored bend target. Record both in `context/re/`. If the drop is not zero, amend the Bend target requirement before coding it. The one-tick test then locks the blend the reading gives.

## 2. Bend the legs

- [x] 2.1 Bend each leg by the law of cosines toward its bend target, smoothed 0.02 or 0.2, visible to the muscles on the next tick. A synthetic two-bone leg at 40 cm hip-to-target gives an unsmoothed knee of 60°, and at 20 cm gives 30°. The same tick's pose is unchanged.
- [x] 2.2 While stepping, turn the thigh 75% of a 40° hip-to-target angle (30°) in the character frame, and decay a 30° extra turn to 27° when not stepping. An anchored stepping human foot at 40 cm, crouch 0, stance factor 0.5, lifts −11.5185 cm. A 10 cm lift on a free foot is 8 cm the next tick.
- [x] 2.3 A standing synthetic body with one foot's anchor on a 15 cm step meets that step with the knee bent, through the shipped body step.

## 3. Retire the stair coupling

- [x] 3.1 Remove the controller-floor raise from the lift. On level ground the pelvis target stays the original's at a 0, 2 and 5 cm float. A controller 12 cm up no longer adds 6 cm. The four-step walk keeps balance at or above 0.9 and its feet on the landing, and the controller floor does not raise the pelvis.

## 4. Docs and check

- [x] 4.1 Update `docs/PLAN.md` §7, `docs/formats/RSQ.md` and the physics crate README: the leg bend, the thigh turn and the step lift are in; the controller-floor stair coupling is gone; gait plants stay not done.
- [x] 4.2 Corpus skipped: no format change. The human stance factor 0.5 is already pinned by the race tests. The exanima01 stair walk was not launched. The synthetic four-step walk is the gate: balance stays at or above 0.9, the feet stay on the landing, and the controller floor does not raise the pelvis. A fall on that room would be the missing gait probe; the coupling is not restored.
- [x] 4.3 `make check` and `openspec validate --all --strict` both pass.
