# Spec Delta

## ADDED Requirements

### Requirement: Sun shadow softness source
Every sun placement under a sky SHALL write the sun light's shadow softness source: twice the cloud excess `(cloudiness − 1.5)·0.4` capped at 0.9 when it is above the float epsilon (else 0), plus `1 − 10·|daylight − 0.44|` while that distance is under 0.1. Without a sky or a sun light the source SHALL be 0. The live frame SHALL hand it to the sun map's region, whose softness and caster bias read it.

#### Scenario: Clear moonlit night
- **WHEN** seed 15 on `arenao3` (cloudiness 1.258) is placed at 0.86, daylight 0.351
- **THEN** the source is 1 − 10·0.089 ≈ 0.11 and the sun map's region carries it

#### Scenario: Cloudier sky
- **WHEN** the same placement is made with cloudiness 2
- **THEN** the source is 0.4 higher

#### Scenario: Indoor level
- **WHEN** an indoor level is lit
- **THEN** the source is 0

### Requirement: View post amount has no effect
The outdoor preset's night value and the indoor arenas' amounts (0.3, 0.6) SHALL be kept as the view's post amount for the log only: the original copies it into its post processor, which no pass or uniform reads, and its post processor exists whether or not the amount passes 0.01, so nothing on screen SHALL depend on it.

#### Scenario: Night draw
- **WHEN** an outdoor arena draws a night
- **THEN** the load logs the amount as stored and never read, and the post chain runs as on a day
