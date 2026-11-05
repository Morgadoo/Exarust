## ADDED Requirements

### Requirement: Spectated match
A match's participants SHALL be the board's lineup in entry order, each company entry standing for its pool slot (0 the manager, roster member `i` slot `i + 1`), whose name, health and participant id its body takes. The first participant the player may control SHALL be the player; when none is (only hired employees are entered), the match start SHALL leave the scene without a player body, start the controls' Spectate mode with the camera looking at the arena's spawn spot, and stand every entrant from the generator. The return to the hub SHALL build the manager again from his record, with his possessions and health, and end the Spectate mode (Character mode again).

#### Scenario: An employee fights alone
- **WHEN** a career with one hired combatant begins a duel with the lineup [1] (`--hub-enter 1`)
- **THEN** the manager is benched, the spectate session starts, the combatant and the opponent are stood and duel each other, and the return to the hub stands the manager again and ends the session

#### Scenario: Entry order maps the roster
- **WHEN** the lineup is [2, 0]
- **THEN** company entry 0 is roster member 2 with id 2 and entry 1 is the manager with id 0
