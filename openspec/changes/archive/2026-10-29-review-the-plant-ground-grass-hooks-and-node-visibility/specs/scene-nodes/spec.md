## ADDED Requirements

### Requirement: Finalise visibility walks
A node whose `flags & 0x300000` is `0x100000` SHALL hide every light node (`flags & 0x5f == 5`) in its tree; `0x200000` SHALL hide light and fire-emitter nodes in its tree when the fade value is above the f32 0.12; the hidden state SHALL be `flags2` bit `0x2`; NaN and exactly 0.12 SHALL leave nodes shown, and `0x300000` SHALL match neither rule.

#### Scenario: Fade at and above the limit
- **WHEN** a fade-rule light with a light child, a hide-rule light, a both-bits light, a plain light and a fade-rule fire are walked at 0.12 and then 0.13
- **THEN** only the hide-rule light is hidden at 0.12, and at 0.13 the fade light, its child and the fire join it
