# Spec Delta

## MODIFIED Requirements

### Requirement: Rain objects
A rain request above zero SHALL create the rain sound and the rain effect when they do not exist, and the weather tick SHALL drive only a created sound: its volume and pitch from the effect while it rains, a start when its volume passes 0.01 and a stop request when a started sound falls below 0.01. On every tick where the rain amount differs from its target, a live rain effect SHALL take the new amount as its intensity.

#### Scenario: No request
- **WHEN** the scene ticks with rain but no request was made
- **THEN** no sound starts

#### Scenario: The rain rises to its target
- **WHEN** a request of 0.9 is made at a rain of 0.89 and the scene ticks 200 ms later
- **THEN** the rain is 0.9 and the effect's intensity is 0.9, and it keeps that intensity while the rain stays at its target

## ADDED Requirements

### Requirement: Weather events reach the game
Each frame the live scene environment SHALL keep the weather tick's requests (stop the rain effect, start or stop the rain sound, a thunderclap) and the tick's game clock for the systems that run after it. A thunderclap SHALL carry the strike's position at the moment it plays.

#### Scenario: A clap after a strike
- **WHEN** a strike 316 cm out is started next to a view and the scene ticks after its thunder time
- **THEN** the tick asks for one clap at gain 4·0.8775 + 0.1 at the strike's position, view offset included
