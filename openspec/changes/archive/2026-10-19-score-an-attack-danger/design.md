# Design

`score_place` keeps the distance terms. A `Blow` carries the arc, the phase, the candidate bearing and the observer's skill. `attack_danger` is the unscaled term. The score then multiplies danger by (skill / 2 + 1/2) × (1 − feral) and the close term by (1 − feral / 2), unless the fighter is dull, which zeros danger and skips both.

A side swing's arrival is the arc gap, or that gap plus the windup while the phase is at most 0.4, times 1000 and rounded to even. A thrust uses the arc test instead of the arrival. `TCombatant` moves from 7% to 11%. The positioning row moves from 40% to 60%. The approach term and the recovering close tweak are not in. The AI section stays 1%. The headline stays 14.4% and 5.7%.
