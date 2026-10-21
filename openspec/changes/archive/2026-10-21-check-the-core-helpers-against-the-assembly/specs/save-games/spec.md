# Spec Delta

## MODIFIED Requirements

### Requirement: Package opening
Opening a package file SHALL create it empty when it is missing and SHALL read a file of eight bytes or fewer, or one with another magic, as a fresh package (the magic, count 1, the sentinel of offset 0 and size 16) without writing anything; the next section write SHALL put the header and directory over the file's start and leave the bytes after them.

#### Scenario: A damaged slot file
- **WHEN** a story scan meets a 7-byte file named Exanima007.rsg
- **THEN** the file keeps its 7 bytes, is not listed and is not deleted

#### Scenario: A missing file
- **WHEN** a new slot is created
- **THEN** its save file exists and is empty

#### Scenario: A later write
- **WHEN** a section is written into a 100-byte file with another magic
- **THEN** its first 56 bytes are the header and two directory entries and bytes 56 to 99 are as they were
