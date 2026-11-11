## ADDED Requirements

### Requirement: Weapon record up the part chain
A striking or struck part SHALL read its own weapon stat record, else its nearest ancestor's below the tree's root, whose record is never read from below. A held item's part SHALL find its top part's record, a fist its hand's, and a striking part with none SHALL deal nothing.

#### Scenario: A child part of a held weapon strikes
- **WHEN** the second part of a held weapon's body meets a limb
- **THEN** the blow uses the record of the weapon's top part

### Requirement: Clash ring on the struck part
A clash SHALL ring the struck item with the record its struck part's chain finds, measured in that part's own frame, and SHALL stamp that part's sound clock and the striking part's.

#### Scenario: A blade strikes a weapon's top part
- **WHEN** a blade clashes with the top part of a held sword
- **THEN** the sword's ring is placed in that part's frame and that part's sound clock is stamped

### Requirement: Weapon mass on the registered body
A held weapon's body SHALL take the holding pass's redistribution when it registers: each part latched at its registered mobility, inflated by 1.25, its link and contact mobilities set to `1/(M·(0.5/n + 0.5·r/Σr))`, and the top part's contact mobility divided by the holder's size. The parts' own links SHALL keep their shares.

#### Scenario: A one-part weapon of table mass 0.25
- **WHEN** a one-part weapon of table mass 0.25 is held by a holder of size 1
- **THEN** its link and contact mobilities are 4

### Requirement: Grip links and held contact from the weapon mass
A held item's grip links SHALL take their shares from the item's link mobility, and the arm setter SHALL scale a held weapon's contact mobility from the top part's contact mobility the redistribution left.

#### Scenario: An attack's held factor
- **WHEN** a held weapon's redistributed top contact mobility is 3 and its holder attacks with a held factor of 1
- **THEN** its contact mobility is 3

### Requirement: Shell damage callback
A wound on flesh SHALL ask the struck body's Force Shell to ripple at the contact with the yellow damage before its 0.75, after the struck behaviour's notice and before the player's feedback.

#### Scenario: A shelled fighter is wounded
- **WHEN** a fighter under Force Shell takes a wound of yellow 0.2
- **THEN** its shell takes a ripple at the contact whose strength comes from 0.2

### Requirement: Clawed hands after an overhead wound
An overhead swing's wound on flesh SHALL put a heavy-clawed striker's two hands back at their base contact mobility, undoing the quarter its attack start gave them.

#### Scenario: A Sir's overhead lands
- **WHEN** a Sir (heavy claws) wounds a fighter with an overhead swing
- **THEN** both its hands' contact mobility return to their base

### Requirement: Worn rules on an equipment change
A body whose possessions change SHALL run the armour pass's worn rules again: the second wind, the item-effect list in wearing order (an item still worn keeping its bytes), the golem suit, the mind ward and the focus charm.

#### Scenario: Taking off a second-wind item
- **WHEN** a fighter takes off its only worn item of special kind 2
- **THEN** it no longer has the second wind

### Requirement: Golem core on an equipment change
A golem race SHALL have its red regeneration blocked unless it wears, without an apparel section, its core: item 500 when its race has flag 0x2000, else item 431 when it has 0x1000; the test SHALL run at spawn and on every equipment change.

#### Scenario: A golem of both kinds
- **WHEN** a golem whose race has both 0x2000 and 0x1000 wears item 431
- **THEN** its red regeneration stays blocked
