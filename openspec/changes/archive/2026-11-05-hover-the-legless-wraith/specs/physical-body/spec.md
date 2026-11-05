# Spec Delta

## ADDED Requirements

### Requirement: Legless hovering body
A living body whose race hovers SHALL run its balance and feet each tick whatever its leg count. With fewer than two legs the balance SHALL only put the feet centre at the pelvis, leaving the balance, its counter and the steps as they are; the hovering feet SHALL give full support; the lift SHALL aim the pelvis at the clip root's height over the character frame less 10 cm, with no ground added; and standing, the support pins SHALL clamp around the pelvis.

#### Scenario: The wraith stands
- **WHEN** a legless hovering body's pose puts its pelvis 100 cm over its frame, over a floor at 0 and then at 30 cm
- **THEN** its target is 90 cm both times, its support and balance stay 1 and its pelvis floats about 92 cm up
