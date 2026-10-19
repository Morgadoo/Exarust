# Spec Delta

## ADDED Requirements

### Requirement: Season rewards
A swept season SHALL grant that season's four item ids. Season 0 grants 1175, 1174, 1177 and 1176. Season 1 grants 1167, 1165, 1168 and 1169. Season 2 grants 1171, 1170, 1173 and 1172. Season 3 grants 1178, 1179, 1180 and 1181. The season is the day divided by 28, wrapped to four, so day 134 grants season 0 again. A season the company did not sweep SHALL grant nothing.

#### Scenario: Four rows, then the first again
- **WHEN** a company sweeps every week through day 134, and another company loses every week through day 22
- **THEN** the first holds 1175, 1174, 1177, 1176, then 1167, 1165, 1168, 1169, then 1171, 1170, 1173, 1172, then 1178, 1179, 1180, 1181, then 1175, 1174, 1177, 1176 again, and the second holds none
