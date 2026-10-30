# Spec Delta

## MODIFIED Requirements

### Requirement: Encounters
An encounter SHALL create the relation at familiarity 0 or renew its time. It SHALL return, in order, every encounter script of A's roles that is for anyone, for B in the current locale, or for one of B's roles, once per matching role. In the running game an encounter the AI's target pass reports SHALL be consumed once, renew the meeting time on the script cast's relation record too, and run each returned script with A as actor 0 and B as actor 1.

#### Scenario: A golem meets a human
- **WHEN** a golem role with encounter scripts for anyone, for character `0x80000002` of locale 2, for Human and for Construct meets human `0x80000002` in locale 2, again later, and in locale 5
- **THEN** the scripts for anyone, the character and Human are returned, the time is renewed, and locale 5 returns two

#### Scenario: A reported encounter runs once
- **WHEN** an observer whose role has an encounter script for anyone has an encounter with a partner reported at 1234 ms, and the system runs twice
- **THEN** the script's effect is written once, and both relation records hold 1234 ms

### Requirement: Roles script functions
The roles unit SHALL register `StartDialogue` and then `ChangeRole`; `StartDialogue` SHALL carry the calling context and the topic named by its first argument; `ChangeRole` SHALL report that the function does not exist.

#### Scenario: Registration
- **WHEN** the registration is listed, `StartDialogue` is asked for topic `rumours` and `ChangeRole` is called
- **THEN** the names are `StartDialogue`, `ChangeRole`, the request holds the context and `rumours`, and the error reads `Function "ChangeRole" does not exist.`
