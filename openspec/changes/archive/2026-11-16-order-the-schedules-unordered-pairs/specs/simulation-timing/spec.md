## ADDED Requirements

### Requirement: The game's schedules leave no conflicting pair unordered
The workspace tests SHALL build the game's App as the binary builds it, headless in lockstep, and fail on any two systems of a fixed-step schedule, of the character tick, of the brains' chain or of `PreUpdate` that touch the same data with no order between them, naming both systems and the data.

#### Scenario: A plugin adds a system that writes the bodies
- **WHEN** a lane adds a fixed-step system that writes `Ragdoll` with no order against the systems that read it
- **THEN** the check fails and names each such pair and `Ragdoll`

### Requirement: Allowed pairs carry a reason
An unordered conflicting pair SHALL pass only through an allowlist entry naming the schedule, both systems, the data and the reason the order cannot change what either computes; an entry that no longer matches SHALL fail the check.

#### Scenario: A stale entry
- **WHEN** an allowed pair is ordered by a later change
- **THEN** the check fails asking to drop the entry

### Requirement: The frame's Update and PostUpdate keep a TODO count
Until they are ordered, `Update` and `PostUpdate` SHALL keep a count of their unordered conflicting pairs in the check, which SHALL fail when the count grows (a new pair) and when it falls (the count to lower).

#### Scenario: A new frame system
- **WHEN** a lane adds an `Update` system that conflicts with another and orders neither
- **THEN** the check fails with the new count

### Requirement: The fixed step's phases after the solver are named
The fixed step SHALL run, after the solver and before the post-step, the work that reads the bodies as the solver left them, the bodies' sounds, the level's mechanisms, the strikes, the blood sprays, then the physics' own follow-up (the grips' lengths, the mechanisms' step, the props, the sector lists), in that order.

#### Scenario: A trace of the bodies
- **WHEN** the body trace records a tick
- **THEN** it reads the bodies after the solver and before the sounds, strikes and mechanisms of that tick write them
