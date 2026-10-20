# Design

## Behaviour

The power keeps a state (`0` ended, `1` held, `2` fading, `3` charging), a value and a clock stamp. Held forces the value to 1. Charging adds `(clock − stamp)/30000` and clamps at 1, switching to held; fading subtracts the same amount and, below 0.002, switches to ended with the value 0 and no publish. The stamp is set to the clock every tick.

While fading, strength is `1000·value` and the smooth/vector outputs are skipped. Otherwise the smooth value is `1 − (1 − value)²`, strength is `(1 − smooth)·rate + 1000·smooth`, and the body vector's scale is `1 − value`. The ended state follows the same publish path with value 0, unless the fade ended this tick, which returns inactive.

## Model

`SpiritForm { state, value, stamp }`, `step_spirit_form(form, clock, rate) -> SpiritStep`. `SpiritStep` carries the state, value, strength, optional smooth and vector scale, and whether the form is active.

## Open questions

The body-field writes and the tail hook belong to the renderer/effect side.

## Docs

`SG_Thaum_Mind` is **2%**. The headline stays 14.4% and 5.7%.
