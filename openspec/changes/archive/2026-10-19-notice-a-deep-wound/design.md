# Design

## Notice

`notices_damage` weighs the lasting wound, the size before the red bar's 0.75, by multiplying by the float 0.75, widening that product, and multiplying by the double 0.2. The line is the float 0.25 widened and multiplied by the double 0.05. The wound is noticed only when the weighed value is greater. An equal value and a non-number are not. `react_to_hits` writes `DamageTaken` when the blow's wound is noticed.

## Open questions

- Limb impairment is not this comparison.
- A low yellow bar does not knock the fighter out from this path.

## Docs

The combat-rule figure stays 27.3%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
