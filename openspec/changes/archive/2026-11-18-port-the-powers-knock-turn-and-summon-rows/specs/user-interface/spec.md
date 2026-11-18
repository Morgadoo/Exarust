## ADDED Requirements

### Requirement: Non-modal message box
A hub message box without the modal bit 0x80 SHALL keep the keys but let a pointer off the box reach the highest form under it; a box with the bit SHALL take every pointer hit.

#### Scenario: The hire refusal box
- **WHEN** a 0x41 box is open over a form and the pointer clicks that form's button beside the box
- **THEN** the form under the box answers and the box stays on top
