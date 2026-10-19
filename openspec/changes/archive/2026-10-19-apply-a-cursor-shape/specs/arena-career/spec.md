## ADDED Requirements

### Requirement: Cursor shape
Shape 0 SHALL show the default image when it differs and is not zero, mark the default, and stamp the clock. Shape 1 and up SHALL use the record one before that index. A different non-zero image SHALL be shown once. The same image SHALL not be shown again. A hook on that record SHALL run. A shape past the table SHALL leave the current image. Drawing the image is not part of this step.

#### Scenario: Shape 7 shows its image and runs its hook
- **WHEN** a match begin restores shape 7 with image 42 and a hook, the same image is restored again, shape 0 restores default image 9, and shape 7 is restored with an empty table
- **THEN** the first shows 42 and runs the hook, the second does not show again and still runs the hook, shape 0 shows 9 and marks the default, and the empty table leaves the current image

### Requirement: Focus call
A focus SHALL be invoked once before the hand words are copied. A session mark that is not already 0 SHALL become 0 after that copy. A mask without a focus SHALL not invoke it. The focus method body and the rest of the session-mark path are not part of this step.

#### Scenario: A focus is invoked before the hands
- **WHEN** a match begin follows the lists with a focus, a session mark of 1, and a ready hand list, and another begin uses mask 1 and no focus
- **THEN** the focus was invoked once, the hands are copied, the session mark is 0, and the mask-only begin does not invoke the focus
