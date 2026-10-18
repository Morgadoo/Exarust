# Spec Delta

## ADDED Requirements

### Requirement: Knockback step
A blow whose impact is over 0.5 SHALL start a knockback step of 2·(20·strikes + 10)·impact centimetres along the horizontal swing, plus 20 ticks of the chest's fast velocity, capped at the step length. Strikes SHALL be 0 until that power exists. Impact of 0.5 or less SHALL NOT start that step.

#### Scenario: A full impact
- **WHEN** impact is 1, strikes are 0, the swing is horizontal and the chest is still
- **THEN** the step is 20 cm along the swing

#### Scenario: Force strikes
- **WHEN** impact is 1 and strikes are 1, with a still chest
- **THEN** the step is 60 cm along the swing

#### Scenario: At the line
- **WHEN** impact is 0.5
- **THEN** no knockback step is started

#### Scenario: A moving chest
- **WHEN** impact is 1, strikes are 0 and the chest moves 1 cm a tick along the swing
- **THEN** the step is 40 cm along the swing
