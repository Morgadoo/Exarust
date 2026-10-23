## MODIFIED Requirements

### Requirement: Combat readiness
The system SHALL raise a fighter's readiness by 0.004 a tick, up to 1, while its stance is Cmbt - Idle or it parries, and its clip is Cmbt - Idle, it parries or its attack is released; else it SHALL reset it to 0. A blow SHALL scale the victim's penetration, impact and stab energy by 1 − 0.5 × its readiness, after the shove is taken from the impact.

#### Scenario: Ready for 3 s
- **WHEN** a fighter that has stood in Cmbt - Idle for 3 s takes a sword cut on bare flesh
- **THEN** the cut's penetration is half what it would be on a fighter in the Defence stance, and the shove is the same

#### Scenario: Readiness through a parry
- **WHEN** a fighter in the Defence stance playing an attack clip parries
- **THEN** its readiness rises by 0.004 a tick, and it resets to 0 once the parry ends

### Requirement: Dazed stance
While a fighter's energy is more than 0.05 under its most (double precision) and it does not parry, its stance SHALL be the Defence stance every body tick, and the player's stance rule SHALL not give Cmbt - Idle. Unless helpless, attacking or shock-staggered, the Defence clip SHALL start when another plays, blend round((1.05 − E) × 300), eased smoothly. A daze is decided before the posture stage, so a fall's clip wins.

#### Scenario: Struck in the combat idle
- **WHEN** the player in combat mode, in Cmbt - Idle, is dazed to energy 0.62
- **THEN** its stance turns to the Defence stance, which starts with a blend of 129 frames, eased smoothly, and does not restart while the daze lasts

#### Scenario: Wits back
- **WHEN** the dazed player's energy is back within 0.05 of its most with its target near
- **THEN** its stance turns to Cmbt - Idle and the posture switch starts it with a blend of 30 frames

#### Scenario: Down or attacking
- **WHEN** a fighter down, getting up or attacking is dazed
- **THEN** its stance turns to the Defence stance and its clip plays on

#### Scenario: Duel
- **WHEN** `exarust --opponent --trace t.csv play arenao1` runs until the duellist kills the player
- **THEN** after each blow the player guards while dazed, and when its energy is back its targets turn under 0.5° a tick (10–18° in one tick without the daze's start)

#### Scenario: Shoved over
- **WHEN** `exarust --shove 60 --trace t.csv play arena` dazes the player to energy 0.13 and it falls
- **THEN** the Defence stance starts (blend 276), Rebalance and then the fall replace it each tick until the player is down, and the get-up ends in Idle without a jump

### Requirement: Shock stagger
While the shock-stagger flag is set, a body tick SHALL set energy to its most minus 0.5 in single precision, set the stance to Defence (when the body has one) without starting its clip, start Rebalance with no blend unless it already plays, and clear the flag once the game clock is past the next shock. The daze's Defence clip SHALL NOT start on that tick. A shock on a fighter that is not collapsing or dead SHALL set the flag.

#### Scenario: The flag holds, then the clock passes
- **WHEN** a fighter with the flag, energy 1 and most 1, plays Idle with the next shock due at 500 ms, and is ticked at 0, 144 and 504 ms
- **THEN** the first tick leaves energy at 0.5, the stance Defence and Rebalance started with blend 0, the flag kept; the second keeps the flag without restarting Rebalance; the third clears the flag

#### Scenario: A body without a stance
- **WHEN** a staggered body that has no stance plays Idle
- **THEN** its energy is pinned and Rebalance starts as for a fighter

## REMOVED Requirements

### Requirement: Bleeding damages over time
**Reason**: The rule was an invented second copy of the burning routine (same constants), wired into every character tick and fed by wounds. The original's body tick has one damage-over-time routine, the burning, fed only by the particle body-contact pass; a wound causes no damage over time.
**Migration**: None. Wounds, sprays and blade blood stay cosmetic; the burning routine (`exa_combat::burn`) is the one damage over time, and a fire's dose reaches it through the limb part's slot (`Burn::add_dose`, for a part with a bone index above zero), whose caller, the particle body-contact pass, is not wired yet.
