## MODIFIED Requirements

### Requirement: Gate hook
The gate hook SHALL clear two result words and set the result flags to 1. A set mid byte with an alternate hook SHALL count that hook. A clear mid byte with a closed world SHALL leave the flags at 1 and SHALL not count the alternate hook.

#### Scenario: A set mid byte counts the alternate hook
- **WHEN** the gate hook runs with empty results, and again with a set mid byte and an alternate hook
- **THEN** both runs set the flags to 1, the first counts nothing, and the second counts the alternate hook

## ADDED Requirements

### Requirement: Cursor pick
A clear mid byte SHALL count a pick when the world is open, the busy byte is 0, the alternate hook is 0, the hold byte is 0, and a set bank side has a world aim. A taken item SHALL publish the notice owner and OR the flags with 5. A set kind SHALL count a kind call. An actor whose parent equals the world SHALL count an open. Kind 0xa00 SHALL count a chest. A named row SHALL count an open. Bit 0x800 SHALL not. A set hold byte or a busy byte SHALL not pick. The raycast is a supplied result.

#### Scenario: A clear gate picks a taken item
- **WHEN** the gate hook runs on an open world with a taken item and notice owner 6, again with the hold byte set, and again with a busy byte
- **THEN** the first ORs the flags with 5 and publishes owner 6, and the other two do not pick

#### Scenario: An actor parent counts an open
- **WHEN** an actor parent equals the world, a kind word is 0xa00, a named row is set, and a row also has bit 0x800
- **THEN** the world parent and the named row each count an open, the kind word counts a chest, and bit 0x800 counts nothing

#### Scenario: A set kind counts a kind call
- **WHEN** an open world picks an item whose kind is set and that is not taken
- **THEN** the kind call is counted once and the flags stay 1

#### Scenario: A set bank side needs a world aim
- **WHEN** the gate hook runs on an open world with bank side 1 and no world aim, and again with a world aim
- **THEN** the first does not pick and the second counts one pick

### Requirement: Click hook
The click hook SHALL clear its flags. An open world whose aim bits are above 0x428c0000 SHALL copy the aim place. A miss with the hold byte set SHALL clear the aimed word. A taken item SHALL set flags that include 0x11. Aim bits equal to 0x428c0000 SHALL not copy the place. The rest of the click path is not part of this step.

#### Scenario: A fast aim copies and a taken item marks the click
- **WHEN** a click runs on an open world with aim bits above 0x428c0000, a miss, and the hold byte, and again with aim bits equal to that value and a taken item
- **THEN** the first copies the place and clears the aimed word, and the second sets flags 0x11 and does not copy

### Requirement: Hover hook
The hover hook SHALL publish a follow hook and the self owner, clear the hover byte and the hover mark, and store the flags 0x15.

#### Scenario: A hover stores flags 0x15
- **WHEN** the hover hook runs with owner 3, byte 4, and mark 6
- **THEN** the flags are 0x15, the follow hook is published for owner 3, and the byte and the mark are 0

### Requirement: Focus before hands
A focus begin SHALL count one call before the hand words are copied. The hands SHALL be set and the stamp SHALL copy the clock. The focus method body is not part of this step.

#### Scenario: A focus begin counts one call
- **WHEN** a focus begin copies the hands at clock 5
- **THEN** the focus is counted once, the hands are set, and the stamp is 5
