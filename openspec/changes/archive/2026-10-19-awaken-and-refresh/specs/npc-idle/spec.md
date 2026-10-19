# Spec Delta

## ADDED Requirements

### Requirement: Awaken and refresh
The tick that drops the dormant flag SHALL tell each person who can see the body, and nobody else. A later tick SHALL tell nobody. When the clip base reaches 1 the reach SHALL be the greatest weapon reach, or the race reach when no hand bit is set. No weapon SHALL prefer thrust 0. A zero slash, crush and pierce sum SHALL prefer 10. Otherwise the preference SHALL be the rounded thrust share times 5, plus 1, and at most 9.

#### Scenario: One witness hears, then the weapons refresh
- **WHEN** a knocked-out body is ticked at 1501 ms with one person who can see, one person who cannot, and one non-person who can, then ticked again at 1502 ms, and a body whose clip base is 0.995 is ticked at 8001 ms with a 40 cm weapon of equal slash and thrust and an 80 cm weapon, and another such body is ticked with no hand bit and a weapon whose slash, crush and pierce are 0
- **THEN** one witness hears at 1501 ms and none at 1502 ms, the armed finish reaches 80 cm and prefers thrust 6, and the empty-handed finish uses the race reach of 50 cm and prefers 10
