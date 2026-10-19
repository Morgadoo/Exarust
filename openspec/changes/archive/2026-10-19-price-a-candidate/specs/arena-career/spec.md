# Spec Delta

## ADDED Requirements

### Requirement: Listed price
A listed price SHALL be the square of one more than half of six skill levels, and that half stops at 5, times 12, plus four fifths of the gear rounded to the nearest copper. The gear SHALL be the sum of the shop line prices. No skills and no gear SHALL cost 12. A purse below the price SHALL refuse before a full roster is considered. A full roster or a second specialist SHALL refuse without spending. A hire that joins SHALL subtract the price and add it to the amount spent.

#### Scenario: Rank, gear, and a short purse
- **WHEN** an empty candidate is priced, a skill total of 2 is priced, a skill total of 10 and of 12 are priced, a gear line of 7 is priced, a two-silver line is priced, a company with 47 copper hires the skill total of 2, a full roster with no copper hires, that roster is given 100 copper and hires again, and a company hires the skill total of 2, then a physician, then a second physician
- **THEN** the prices are 12, 48, 432, 432, 18 and 31, the short purse stays at 47 with an empty roster, the empty purse is refused with the roster still full, the second try is refused with 100 copper and the spent total still 84, the skill total of 2 leaves 295 copper and 48 spent, and the second physician is refused with the spent total still 60
