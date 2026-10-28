## ADDED Requirements

### Requirement: Trailing-foot request
A clip start that replaces Walk or Run SHALL leave a trailing-foot request with a deadline 500 ms on, unless the movement decision moves the body that tick. The body tick SHALL plant the trailing foot after its sample while the request is 2 and set it to 1. A request strictly past its deadline SHALL be dropped and the behaviour told; the movement decision's move branch SHALL drop it.

#### Scenario: Leaving the walk
- **WHEN** a standing body's Walk is replaced by an attack at 1000 ms
- **THEN** the next sample plants its trailing foot, the request stays 1 at 1500 ms and is dropped at 1501 ms

#### Scenario: Moving
- **WHEN** a body with a request is moved by the movement decision
- **THEN** its request is 0

## MODIFIED Requirements

### Requirement: Femur posture
While the behaviour is in combat and the condition is neither off its feet nor walking, the thighs SHALL be turned about the axis (0, 0, 1) lerped 0.4 toward (-1, 0, 0) by (1 - record height) times -1.5 degrees; otherwise they SHALL be turned 5 degrees with the state word's splay bit, else keep the clip's turn; the sign alternates by leg.

#### Scenario: Standing
- **WHEN** the behaviour is not in combat and the splay bit is clear
- **THEN** the thighs keep the clip's turn

#### Scenario: Walking in combat
- **WHEN** the behaviour is in combat and the condition is walking
- **THEN** the thighs keep the clip's turn
