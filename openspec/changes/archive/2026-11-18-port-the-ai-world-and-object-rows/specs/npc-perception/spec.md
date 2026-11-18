## ADDED Requirements

### Requirement: Force effects assess the whole contact
A brain's sight of a blast, a wave or a barricade SHALL start from the brain's contact for that effect and write back every word the assessment sets: the noticed level (only raised), the sighting (the near score), the concern target (the reach word, none for a wave), the soothe term (the relation word, cleared by a blast and a wave), the alarm (the fear), the seen word (the urge) and the stamp (the face word). Words the assessment leaves SHALL keep the contact's values.

#### Scenario: A released blast three metres ahead
- **WHEN** a brain with a contact for a released blast 3 m ahead sees it
- **THEN** its contact's noticed level, sighting, concern target, alarm, seen word and stamp equal the blast assessment's words and its tolerance is unchanged

### Requirement: Force effects read the body's look
The face word of a force effect's assessment SHALL take its squared term from the body's look direction (the look the brain's gaze last wrote into the body, the frame forward until it first turns) and its linear term from the character frame's forward.

#### Scenario: Looking away from a blast
- **WHEN** two brains face a blast and one's gaze looks the other way
- **THEN** the one looking away has the larger face word

### Requirement: A busy activity's notice floor
While the brain's idle activity answers busy, the target pass SHALL start its notice score at the activity's floor, 4 for the search, so a contact scoring no more than that is not attended; the goal entity under the pick flag SHALL replace it, and the idle restart SHALL clear the busy flag.

#### Scenario: A searching follower and a passer-by
- **WHEN** a brain whose search is busy notices a character with a notice score of 2
- **THEN** it attends no one, and an idle brain attends that character
