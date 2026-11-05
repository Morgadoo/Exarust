# Spec Delta

## ADDED Requirements

### Requirement: Script ShowInventory of a character
`Actor[n].ShowInventory` SHALL open the inventory window of that actor's character without the toggle: the player's own for the player, that character's for anyone else.

#### Scenario: The Equipment topic
- **WHEN** a dialogue node runs `Actor[1].ShowInventory;` with a companion as `Actor[1]`
- **THEN** the companion's inventory window opens, and an already active one stays open

### Requirement: Shipped interaction conditions
In `charroles.rdb` of the installed build exactly two entries SHALL carry an interaction condition: the Golem Boss's toward the Player role (`result = Actor[1].HasItem($1DE);`) and the Golem Servant's toward the Leader role (`result = 1;`).

#### Scenario: Derrin
- **WHEN** the player interacts with Derrin, who carries no marker of the player's
- **THEN** no condition is found and no window opens; with the player's Loyalty marker his inventory window opens
