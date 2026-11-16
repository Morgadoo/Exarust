## 1. The oracle

- [x] 1.1 The case file reader and the replay with field-by-field mismatch reports
- [x] 1.2 The registry of ports by name, for 33 original functions
- [x] 1.3 The replay test, ignored unless a case directory is given

## 2. Findings

- [x] 2.1 The floor and ceiling conversions read a denormal as zero

## 3. Tests and checks

- [x] 3.1 Unit tests: the case file parses, a replay reports the first differing case, the names are unique, a denormal floors and ceils as zero
- [x] 3.2 fmt, clippy -D warnings, the workspace tests, openspec validate
