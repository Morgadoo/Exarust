# Spec Delta

## ADDED Requirements

### Requirement: Description phrase
A phrase whose low and high include the value SHALL be a candidate, in table order. No candidate SHALL leave the roll unchanged and add nothing. Otherwise the roll SHALL become that roll times 0x08088405 plus 1, and the high half of that times the count SHALL choose one. A repeated word longer than three letters, ignoring the last character and letter case, SHALL add nothing. A comma phrase SHALL gain a dot. Any other phrase SHALL gain a leading space and a dot.

#### Scenario: A repeated word stays out
- **WHEN** value 11 misses a row of 0 to 10 from roll 7, value 5 takes "fine" from roll 0, roll 16 chooses between "alpha" and "beta", "fine x" and "Fine x" meet "fine work", "fin x" and "fine" meet "fine work", a comma phrase is chosen, and "the x" meets "the item here"
- **THEN** the miss keeps roll 7 and adds nothing, "fine" becomes " fine." at roll 1, roll 16 becomes 0x80884051 and " beta.", both "fine" texts add nothing, "fin x" and "fine" become " fine work.", the comma phrase becomes ", but torn.", and "the x" becomes " the item here."
