## ADDED Requirements

### Requirement: Settings page switch
The system SHALL show Settings at (0,64,426,408) on page 0 and Controls at y 56 (widescreen) or 96 with height 472−y on page 1, scaling by unit/4.

#### Scenario: The two pages
- **WHEN** page 0 is switched at unit 4
- **THEN** Settings is visible and its child is (0,64,426,408)
- **WHEN** page 1 is switched widescreen at unit 4
- **THEN** Controls is visible at (0,56,426,416)
- **WHEN** page 1 is switched non-widescreen
- **THEN** the child is (0,96,426,376)

### Requirement: Controls scrollbar
The system SHALL size the Controls thumb to 232 px and place it at `17 + 150·f` over the 412-px client and 676-px content at unit 4.

#### Scenario: Ends
- **WHEN** the fraction is 0
- **THEN** the thumb is 232 at y 17
- **WHEN** the fraction is 1
- **THEN** the thumb is at y 167
