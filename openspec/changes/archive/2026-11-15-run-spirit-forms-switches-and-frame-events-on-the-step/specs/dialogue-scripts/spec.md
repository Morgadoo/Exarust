## ADDED Requirements

### Requirement: Contact write-back behind the bridge
Behind the simulation bridge the contact mirror and the greeting SHALL post a script's or a line's contact write to the simulation's brain through the step's write queue, and SHALL write the frame's copy without a change mark. A mirror that writes nothing SHALL leave the brain unmarked.

#### Scenario: A script write behind the bridge
- **WHEN** a script sets a contact's concern while the simulation runs behind the bridge
- **THEN** the frame's brain shows it at once, the frame's brain is not sent, and the simulation's brain takes the write at its next step
