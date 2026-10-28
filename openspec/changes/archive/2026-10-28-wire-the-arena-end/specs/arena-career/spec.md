## ADDED Requirements

### Requirement: Drawn arena end screen
When a round ends with a transition screen, the game SHALL draw the arena's single-row end menu through the GUI path, `NEXT FIGHT` for screen 3 and `EXIT;` for screen 2 at scale 0.9, and SHALL run the Continue only from that menu's row (Enter or a pointer release over it); the arena pause page's FORFEIT SHALL settle a fighting match as forfeited and return to the hub, and SHALL be ignored when the manager is down.

#### Scenario: Space no longer continues
- **WHEN** a won match's end screen is open and Space is pressed, then Enter
- **THEN** Space does nothing and Enter settles the match and asks for the hub

### Requirement: Tournament next fight
A tournament's Continue after a won round SHALL resolve the bracket with the player's team kept and every other group's winner drawn, and SHALL stand the first surviving teams in slot order, as many as the event has sides, with the role flag hostile below three teams and free-for-all otherwise.

#### Scenario: A duel event stays a duel
- **WHEN** the four-round duel tournament continues after its first won round, with eight teams surviving
- **THEN** exactly one opponent is raised, generated from its record, with the hostile role, and the player is placed on the ring

### Requirement: Generated arena records
A generic or hire-list arena opponent SHALL be built from its own generated character record (random NPC pass, then the level or profile outfit pass, all draws from one sequence) with personality −100, 100, 0 and, for a hire-list opponent, the event's drawn weapon in the main hand, through the placed characters' merged-mesh spawn path.

#### Scenario: Hire personality
- **WHEN** a hire-list fighter is made from a tier 3, company 2 plan without a weapon row
- **THEN** its profile flags are 0x0b with company bytes 66, 140, 1 and its personality is −100, 100, 0, 0
