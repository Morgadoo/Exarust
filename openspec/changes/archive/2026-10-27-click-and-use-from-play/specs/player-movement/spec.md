# Spec Delta

## ADDED Requirements

### Requirement: Click on what is under the cursor
A click outside combat mode SHALL name what it hits as the original's click handler does: a placed object with no usable section SHALL show the item; a lock SHALL open its container when open, and otherwise, with the key and its reward due, pay 100 experience, or 1000 for the lock subtype 3; usable kinds 0 and 1 SHALL open the container and any other kind SHALL show the item. A click on the player's own body SHALL show the health and focus bars for 5000 ms.

#### Scenario: Own body
- **WHEN** the player clicks its own body outside combat mode at game time 1000 ms
- **THEN** the HUD bars are held shown until 6000 ms

#### Scenario: Lock subtype 3
- **WHEN** a closed lock of subtype 3 is clicked with its key and its reward due
- **THEN** the click pays 1000 experience

### Requirement: Double-click interaction
A double click SHALL be a second press of the same button within the double-click time (500 ms) and under 2 pixels of the first; it SHALL interact only outside combat mode or with Ctrl held. On a placed object the hand-held item (kind 1, unmarked) SHALL end the interaction; every other branch SHALL go on to open a container section. The player's own body SHALL open the inventory.

#### Scenario: Consumable in a container
- **WHEN** an unmarked consumable of kind 0 subtype 1 with a container section is double-clicked at 1000 ms
- **THEN** it is consumed at 3100 ms and its container opens

### Requirement: Item use marks and timed values
A repair SHALL mark the item or its container entry with 0xFF0000 and both pours with 0xC80000; the elixir's sound SHALL play only with the potion. The timed list SHALL write a section only while its byte lowers or the entry arrives, and SHALL skip the weapon section in a frame where the usable section exists and is not written.

#### Scenario: Usable section not lowered
- **WHEN** a repair entry's value is 20 and the usable section's byte is 10 while the weapon section's is 90
- **THEN** nothing is written that frame
