# Spec Delta

## Purpose
One cloth tick: how a particle falls, how wind and a pin move it, and how an edge corrects.

## ADDED Requirements

### Requirement: Fall
A particle at rest SHALL drop by the body's gravity for one tick, in centimetres. Its previous position SHALL be where it was.

#### Scenario: Resting vertex
- **WHEN** a particle at rest is stepped with the cloth gravity
- **THEN** its height falls by that gravity and its previous height is unchanged

### Requirement: Edge
An edge at its rest length SHALL not move its ends. A stretched edge SHALL pull its ends closer.

#### Scenario: Rest and stretch
- **WHEN** an edge of length 20 joins particles 20 apart, then one of length 10 joins particles 20 apart
- **THEN** the first pair stays put and the second pair is closer than 20

### Requirement: Pin and wind
A pin of weight 1 SHALL place its particle on the rest point. Wind above 0.001 SHALL move a particle by direction times (noise + 0.25) times the wind. Wind at 0.001 SHALL not.

#### Scenario: Held and blown
- **WHEN** a pin of weight 1 has its rest 5 cm along X, then a particle is blown by wind 1 along X with noise 0, then by wind 0.001
- **THEN** the pinned particle sits at 5 cm, the blown one moves 0.25 cm, and the last one stays
