## ADDED Requirements

### Requirement: AI events refresh
The contact the target pass's round-robin refresh reaches SHALL get the dialogue's AI events about it, with the contact's event word as the mask; the bit of each event node that ran SHALL leave the word.

#### Scenario: An Engage event
- **WHEN** the refresh reaches a contact whose event word is 2 and 8 and the brain's dialogue has an Engage node only
- **THEN** the Engage node's action runs and the word keeps 8
