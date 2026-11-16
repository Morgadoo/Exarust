## ADDED Requirements

### Requirement: The walk's mode word and its passed body
The route SHALL keep the walk's mode word (the follow's 4 while following its leader, 6 while closing on it), the leader's point the follow planned from and the body the walk lets pass; the route stop and the soft stop SHALL clear the mode word, and the in-the-way test SHALL skip the passed body. The follow state SHALL read and write its mode and planned point there and SHALL keep its route while the combat state fights.

#### Scenario: A stop clears the mode
- **WHEN** a follower's walk to its leader is soft-stopped
- **THEN** its route is not followed and its mode word is 0

### Requirement: The head looks at the attended contact
The AI's look target the walk's head look reads SHALL be the contact the brain's target pass attends; a walker without a brain SHALL keep the character's own look target.

#### Scenario: Attending, then not
- **WHEN** a walking brain attends a character 45° to its right, and another attends no one
- **THEN** the first head's offset target is that yaw and the second's 0
