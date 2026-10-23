## MODIFIED Requirements

### Requirement: Live limb impact
A sounded limb impact that is not dropped SHALL take damage (volume − 0.25)·m, times 1 − s while the root sinks, less the impact damage taken; a positive excess SHALL add to the taken damage, take half as yellow health (vital over 0.6 of total) and daze by half. Then, hurt or not, the root's sound clock SHALL move to 220 + rand(100) − round(170·volume) ms on, the draw made after the damage step. The taken damage SHALL fall 0.01 a tick while over 0.01, except on a dead body.

#### Scenario: A thrown ball hits an arm
- **WHEN** a fast ball strikes a fighter's limb
- **THEN** the limb takes impact damage and the root's clock waits before the next

## ADDED Requirements

### Requirement: Limb impact gates
A struck limb of a fighter SHALL sound only when the other side is not a striking limb, the root's sound clock has passed and the impact, a third (double precision) unless the fighter is down and then halved, reaches 0.5; the volume is that times 0.05, at most 1. After the sound, a hard impact (volume over 0.25) on a part striking in an overhead swing SHALL be dropped: no damage, no clock, no draw.

#### Scenario: An overhead striker's pelvis
- **WHEN** a fast ball strikes the pelvis of a fighter in an overhead swing whose pelvis is striking
- **THEN** it sounds once at a volume over 0.25, the taken damage stays 0, the clock stays 0 and no impact message is sent
