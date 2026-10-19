## ADDED Requirements

### Requirement: Hook list
A cleared mark SHALL register code 0x8001 when the hold byte is 0, and SHALL register nothing when it is not. A code with a clear high nibble SHALL gain bit 0x1000. A non-zero world byte SHALL register the wide hook at code 0x2001 and at the small code, then drop the plain and lab hooks. Raising the mark SHALL drop the plain, lab, wide, and gate hooks, drop the play hook, and insert the watch hook at key 4. An empty list starts one below 0.

#### Scenario: A cleared mark registers the gate hook
- **WHEN** a match begin clears a mark with hold 0 and gate hook 11, another begin holds the byte, and a third uses world byte 5
- **THEN** the first list is code 0x8001, the held begin registers nothing, and byte 5 stores the wide hook at 0x2001 and at 0x1002 before the gate hook

#### Scenario: Raising the mark drops the hooks
- **WHEN** the primary list holds the plain, lab, and gate hooks and the ordered list holds the play hook
- **THEN** the primary list is empty and the ordered list is the watch hook at key 4

### Requirement: Action set
A shown value SHALL activate the action set with the two platform words. The same value SHALL not activate it again. The Steam library is not loaded.

#### Scenario: Value 42 activates the action set
- **WHEN** a match begin shows value 42 with owner 7 and extra 8
- **THEN** the action set is activated once and those three words are the draw
