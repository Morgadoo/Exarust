## ADDED Requirements

### Requirement: Cursor restore
Releasing a capture that is not the last SHALL publish the remaining top's code and name. Bit 0x8000 on that code SHALL pass its low byte as the shape and SHALL not mark the default. Any other code SHALL pass shape 0 and mark the default. An empty list SHALL publish code 0 and no holder, and SHALL not mark the default. The shape table is not part of this step.

#### Scenario: The lower capture is published
- **WHEN** a match begin releases a top capture and leaves a code of 0x15, another leaves a code of 0x8007, and a third releases the only capture
- **THEN** the first publishes code 0x15, the lower name, and shape 0 with the default marked, the second publishes code 0x8007 and shape 7 without the default, and the third publishes code 0 and no holder without marking the default

### Requirement: List follow
After the two lists, argument 1 SHALL run when a focus is set or when the mask shares bit 0. The mode word SHALL keep bit 1 and then set bit 0, and the stamp SHALL copy the clock. A ready hand list SHALL copy the ten stored words. A focus SHALL latch and clear the pending byte. No focus and a mask of 0 SHALL change nothing. The virtual call is not part of this step.

#### Scenario: Mask 1 copies the ten words
- **WHEN** a match begin follows the lists with mask 1, clock 99, mode word 6, and a ready hand list, and another begin uses mask 0 and no focus, and a third uses a focus with mask 0
- **THEN** the first copies 0x101, 0x20101, 0x102, 0x105, 0x10106, 0x107, 0x2010b, and three zeros, the mode word is 3, and the stamp is 99, the second leaves the hands unset, and the third latches and clears the pending byte
