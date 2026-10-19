## ADDED Requirements

### Requirement: Specialist leave
Before a match the company SHALL walk the roster from the start and skip every combatant. The first specialist with health under 700 SHALL be removed and the death count SHALL rise by one. Health from 700 through 64999 SHALL be removed without that rise. Health of 65000 or more SHALL stay. A later specialist SHALL stay on that pass. An empty roster SHALL remove nobody. Releasing a kit and unlinking a world character are not part of this step.

#### Scenario: The first unhealthy specialist leaves
- **WHEN** an empty company is asked, a hired physician at health 699 is asked, a hired merchant at 700 is asked, a hired trainer at 65000 is asked, a hired combatant at 0 stands before a hired physician at 699, and a hired physician at 100 stands before a hired merchant at 100
- **THEN** the empty company removes nobody, the physician at 699 is gone and the death count is 1, the merchant at 700 is gone and the death count stays 0, the trainer at 65000 stays, the combatant at 0 stays while that physician is removed, and only the physician of the pair is removed
