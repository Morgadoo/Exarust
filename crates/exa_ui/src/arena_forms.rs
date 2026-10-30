//! The arena company windows' control tables, as their constructors hand
//! them to the form's table builder, and the forms' own words.
//!
//! Each array is one constructor's 0x68-byte records (the element counts
//! come from the arrays' own type records; the constructors pass the last
//! index): Manage Company 18 (0x2c4 × 0x1d4, placement 0xb800, the base
//! attribute word), Company Records 27 (0x164 × 0x1d4) and Company Status 12
//! (0x164 × 0x1ec), both with the upper-layer bit 0x1000; the loadout form
//! 28 (1024 × 768, attribute word 0x802022, placement 0 and border 0); the
//! dismiss and yes/no dialogs 3 each (0x188 × 0x70, centred, upper layer,
//! border 0, the dismiss dialog titled "Dismiss", the yes/no untitled).
//! Manage Company also builds its two tables as child panes at (0, 0x18)
//! and (0x15c, 0x18), each 0x158 × 0x128.
//!
//! The records' words are copied from the arrays as stored
//! ([`FormRecord`]); a record whose callback is set reports its changes.
//! The forms are built by [`crate::record_form`].
//!
//! Evidence: the six constructors and the arrays they pass (`docs/UI.md`,
//! Arena company windows).

use crate::creation_forms::{FormRecord, record};
use crate::record_form::FormSpec;
use crate::settings::Rect;

/// Manage Company's 18 records: the Roster and Report titles, Manage,
/// Arsenal, Recruit and Hire, the Owned, Upkeep, Earned today and Spent
/// today labels each with its coin control, End Day (disabled), Close,
/// Records and Status.
pub const MANAGE_RECORDS: [FormRecord; 18] = [
    FormRecord {
        text: Some("Roster"),
        style: 0x3300,
        ..record(0x5, 0x0, 0x0, 0x1, 0x158, 0x1c)
    },
    FormRecord {
        text: Some("Report"),
        style: 0x3300,
        ..record(0x5, 0x0, 0x15c, 0x8000, 0x158, 0x1c)
    },
    FormRecord {
        text: Some("Manage"),
        style: 0x20403300,
        reports: true,
        ..record(0x1, 0x0, 0x28, 0x54, 0x78, 0x1c)
    },
    FormRecord {
        text: Some("Arsenal"),
        style: 0x20403300,
        reports: true,
        ..record(0x1, 0x0, 0xb2, 0x8000, 0x78, 0x1c)
    },
    FormRecord {
        text: Some("Recruit"),
        style: 0x20403300,
        reports: true,
        ..record(0x1, 0x0, 0x28, 0x8005, 0x78, 0x1c)
    },
    FormRecord {
        text: Some("Hire"),
        style: 0x20403300,
        reports: true,
        ..record(0x1, 0x0, 0xb2, 0x8000, 0x78, 0x1c)
    },
    FormRecord {
        text: Some("Owned"),
        style: 0x1300,
        ..record(0x5, 0x0, 0x198, 0x53, 0x0, 0x0)
    },
    FormRecord {
        amount: 0x6311413,
        ..record(0xac03, 0x0, 0x800a, 0x8000, 0x30, 0x0)
    },
    FormRecord {
        text: Some("Upkeep"),
        style: 0x1300,
        ..record(0x5, 0x0, 0x26c, 0x8000, 0x0, 0x0)
    },
    FormRecord {
        amount: 0x406,
        ..record(0xac03, 0x0, 0x8009, 0x8000, 0x30, 0x0)
    },
    FormRecord {
        text: Some("Earned today"),
        style: 0x1300,
        ..record(0x5, 0x0, 0x1f4, 0x8008, 0x0, 0x0)
    },
    FormRecord {
        amount: 0x30406,
        ..record(0xac03, 0x0, 0x8009, 0x8000, 0x30, 0x0)
    },
    FormRecord {
        text: Some("Spent today"),
        style: 0x1300,
        ..record(0x5, 0x0, 0x1f4, 0x8009, 0x0, 0x0)
    },
    FormRecord {
        amount: 0x508,
        ..record(0xac03, 0x0, 0x8009, 0x8000, 0x30, 0x0)
    },
    FormRecord {
        text: Some("End Day"),
        style: 0x483300,
        state: 0x10,
        reports: true,
        ..record(0x1, 0x0, 0x178, 0x2, 0x8c, 0x1c)
    },
    FormRecord {
        text: Some("Close"),
        style: 0x483300,
        reports: true,
        ..record(0x1, 0x0, 0x8008, 0x8000, 0x8c, 0x1c)
    },
    FormRecord {
        text: Some("Records"),
        style: 0x483300,
        reports: true,
        ..record(0x1, 0x0, 0x28, 0x3, 0x78, 0x1c)
    },
    FormRecord {
        text: Some("Status"),
        style: 0x483300,
        reports: true,
        ..record(0x1, 0x0, 0xb2, 0x8000, 0x78, 0x1c)
    },
];

/// Company Records' 27 records: the thirteen labels, each with its value
/// (the last two coin controls), and CLOSE.
pub const RECORDS_RECORDS: [FormRecord; 27] = [
    FormRecord {
        text: Some("Matches won"),
        ..record(0x5, 0x0, 0x28, 0x4, 0x50, 0x0)
    },
    FormRecord {
        text: Some("23"),
        style: 0x4300,
        ..record(0x5, 0x0, 0xb4, 0x8000, 0x50, 0x1c)
    },
    FormRecord {
        text: Some("Matches lost"),
        ..record(0x5, 0x0, 0x28, 0x8008, 0x50, 0x0)
    },
    FormRecord {
        text: Some("2"),
        style: 0x4300,
        ..record(0x5, 0x0, 0xb4, 0x8000, 0x50, 0x0)
    },
    FormRecord {
        text: Some("Matches forfeited"),
        ..record(0x5, 0x0, 0x28, 0x8008, 0x50, 0x0)
    },
    FormRecord {
        text: Some("4"),
        style: 0x4300,
        ..record(0x5, 0x0, 0xb4, 0x8000, 0x50, 0x0)
    },
    FormRecord {
        text: Some("Tournaments won"),
        ..record(0x5, 0x0, 0x28, 0x8006, 0x50, 0x0)
    },
    FormRecord {
        text: Some("2"),
        style: 0x4300,
        ..record(0x5, 0x0, 0xb4, 0x8000, 0x50, 0x0)
    },
    FormRecord {
        text: Some("Tournaments lost"),
        ..record(0x5, 0x0, 0x28, 0x8008, 0x50, 0x0)
    },
    FormRecord {
        text: Some("1"),
        style: 0x4300,
        ..record(0x5, 0x0, 0xb4, 0x8000, 0x50, 0x0)
    },
    FormRecord {
        text: Some("Seasons won"),
        ..record(0x5, 0x0, 0x28, 0x8008, 0x50, 0x0)
    },
    FormRecord {
        text: Some("0"),
        style: 0x4300,
        ..record(0x5, 0x0, 0xb4, 0x8000, 0x50, 0x0)
    },
    FormRecord {
        text: Some("Beasts fought"),
        ..record(0x5, 0x0, 0x28, 0x8006, 0x50, 0x0)
    },
    FormRecord {
        text: Some("3"),
        style: 0x4300,
        ..record(0x5, 0x0, 0xb4, 0x8000, 0x50, 0x0)
    },
    FormRecord {
        text: Some("Beasts defeated"),
        ..record(0x5, 0x0, 0x28, 0x8008, 0x50, 0x0)
    },
    FormRecord {
        text: Some("1"),
        style: 0x4300,
        ..record(0x5, 0x0, 0xb4, 0x8000, 0x50, 0x0)
    },
    FormRecord {
        text: Some("Total Hires"),
        ..record(0x5, 0x0, 0x28, 0x8006, 0x50, 0x0)
    },
    FormRecord {
        text: Some("1"),
        style: 0x4300,
        ..record(0x5, 0x0, 0xb4, 0x8000, 0x50, 0x0)
    },
    FormRecord {
        text: Some("Deaths"),
        ..record(0x5, 0x0, 0x28, 0x8008, 0x50, 0x0)
    },
    FormRecord {
        text: Some("1"),
        style: 0x4300,
        ..record(0x5, 0x0, 0xb4, 0x8000, 0x50, 0x0)
    },
    FormRecord {
        text: Some("Fired"),
        ..record(0x5, 0x0, 0x28, 0x8008, 0x50, 0x0)
    },
    FormRecord {
        text: Some("1"),
        style: 0x4300,
        ..record(0x5, 0x0, 0xb4, 0x8000, 0x50, 0x0)
    },
    FormRecord {
        text: Some("Total earned"),
        ..record(0x5, 0x0, 0x28, 0x8006, 0x50, 0x0)
    },
    FormRecord {
        text: Some("1"),
        style: 0x4300,
        amount: 0x6311413,
        ..record(0xac03, 0x0, 0xc0, 0x8000, 0x50, 0x0)
    },
    FormRecord {
        text: Some("Total spent"),
        ..record(0x5, 0x0, 0x28, 0x8008, 0x50, 0x0)
    },
    FormRecord {
        text: Some("1"),
        style: 0x4300,
        amount: 0x23413,
        ..record(0xac03, 0x0, 0xc0, 0x8000, 0x50, 0x0)
    },
    FormRecord {
        text: Some("CLOSE"),
        style: 0x483b00,
        reports: true,
        ..record(0x1, 0x0, 0x24, 0x2, 0xa4, 0x1c)
    },
];

/// Company Status's 12 records: the company's name, Renown, Reputation,
/// Morale, Total Staff and Masters each with its value, and CLOSE.
pub const STATUS_RECORDS: [FormRecord; 12] = [
    FormRecord {
        text: Some("The Jolly Ranchers"),
        style: 0x3b00,
        ..record(0x5, 0x0, 0x4, 0x38, 0x50, 0x0)
    },
    FormRecord {
        text: Some("Renown"),
        ..record(0x5, 0x0, 0x38, 0x8004, 0x50, 0x0)
    },
    FormRecord {
        text: Some("Unknown"),
        style: 0x3300,
        ..record(0x5, 0x0, 0xbc, 0x8000, 0x50, 0x1c)
    },
    FormRecord {
        text: Some("Reputation"),
        ..record(0x5, 0x0, 0x38, 0x8007, 0x50, 0x0)
    },
    FormRecord {
        text: Some("Unknown"),
        style: 0x3300,
        ..record(0x5, 0x0, 0xbc, 0x8000, 0x50, 0x0)
    },
    FormRecord {
        text: Some("Morale"),
        ..record(0x5, 0x0, 0x38, 0x8007, 0x50, 0x0)
    },
    FormRecord {
        text: Some("Good"),
        style: 0x3300,
        ..record(0x5, 0x0, 0xbc, 0x8000, 0x50, 0x0)
    },
    FormRecord {
        text: Some("Total Staff"),
        ..record(0x5, 0x0, 0x38, 0x8005, 0x50, 0x0)
    },
    FormRecord {
        text: Some("13"),
        style: 0x3300,
        ..record(0x5, 0x0, 0xbc, 0x8000, 0x50, 0x0)
    },
    FormRecord {
        text: Some("Masters"),
        ..record(0x5, 0x0, 0x38, 0x8007, 0x50, 0x0)
    },
    FormRecord {
        text: Some("None"),
        style: 0x3300,
        ..record(0x5, 0x0, 0xbc, 0x8000, 0x50, 0x0)
    },
    FormRecord {
        text: Some("CLOSE"),
        style: 0x483b00,
        reports: true,
        ..record(0x1, 0x0, 0x24, 0x2, 0xa4, 0x1c)
    },
];

/// The loadout form's 28 records: the ARSENAL title, the fighter list (a
/// combo box), Previous, Next and Dismiss, the Rank label and slider (five
/// steps), the points line, Encumbrance and its dots, a panel, the five
/// shelf image buttons (group 16) and the five box buttons (group 32),
/// CLOSE, the hidden HIRE with its fee and upkeep, and the hidden
/// profession line.
pub const LOADOUT_RECORDS: [FormRecord; 28] = [
    FormRecord {
        text: Some("ARSENAL"),
        style: 0x3b00,
        ..record(0x5, 0x0, 0xc8, 0x1, 0xa0, 0x0)
    },
    FormRecord {
        style: 0x8403320,
        reports: true,
        ..record(0x12, 0x0, 0x8, 0x5, 0xdc, 0x0)
    },
    FormRecord {
        text: Some("Previous"),
        style: 0x403320,
        reports: true,
        ..record(0x1, 0x0, 0x4, 0x8006, 0x4c, 0x18)
    },
    FormRecord {
        text: Some("Next"),
        style: 0x403320,
        reports: true,
        ..record(0x1, 0x0, 0x8009, 0x8000, 0x4c, 0x18)
    },
    FormRecord {
        text: Some("Dismiss"),
        style: 0x403320,
        reports: true,
        ..record(0x1, 0x0, 0x8008, 0x8000, 0x40, 0x18)
    },
    FormRecord {
        text: Some("Rank"),
        ..record(0x5, 0x0, 0x8, 0x8005, 0x28, 0x0)
    },
    FormRecord {
        group: 8,
        text: Some("Inept"),
        style: 0x405320,
        icon: 80,
        steps: 5,
        reports: true,
        ..record(0x4, 0x102, 0x38, 0x8000, 0x70, 0x0)
    },
    FormRecord {
        text: Some("Points: 0 / 5"),
        ..record(0x5, 0x0, 0x34, 0x8008, 0xe4, 0x1c)
    },
    FormRecord {
        text: Some("Encumbrance:"),
        ..record(0x5, 0x0, 0x8, 0x8008, 0xc, 0x1c)
    },
    record(0xac01, 0x0, 0x64, 0x8000, 0xc, 0x1c),
    FormRecord {
        style: 0x3300,
        ..record(0x5, 0x0, 0x4, 0x69, 0xe4, 0x0)
    },
    FormRecord {
        group: 16,
        style: 0x1420080,
        icon: 5,
        tag: 4,
        reports: true,
        ..record(0x3, 0x0, 0xc, 0x4, 0x2c, 0x2c)
    },
    FormRecord {
        group: 16,
        style: 0x1420080,
        icon: 4,
        tag: 3,
        reports: true,
        ..record(0x3, 0x0, 0x800a, 0x8000, 0x2c, 0x2c)
    },
    FormRecord {
        group: 16,
        style: 0x1420080,
        icon: 3,
        tag: 2,
        reports: true,
        ..record(0x3, 0x0, 0x800a, 0x8000, 0x2c, 0x2c)
    },
    FormRecord {
        group: 16,
        style: 0x1420080,
        icon: 2,
        tag: 1,
        reports: true,
        ..record(0x3, 0x0, 0x800a, 0x8000, 0x2c, 0x2c)
    },
    FormRecord {
        group: 16,
        style: 0x1420080,
        state: 0x60,
        icon: 1,
        reports: true,
        ..record(0x3, 0x0, 0x800a, 0x8000, 0x2c, 0x2c)
    },
    FormRecord {
        group: 32,
        text: Some("5"),
        style: 0x1423300,
        tag: 4,
        reports: true,
        ..record(0x1, 0x0, 0x10, 0x8004, 0x24, 0x0)
    },
    FormRecord {
        group: 32,
        text: Some("4"),
        style: 0x1423300,
        tag: 3,
        reports: true,
        ..record(0x1, 0x0, 0x8008, 0x8000, 0x24, 0x0)
    },
    FormRecord {
        group: 32,
        text: Some("3"),
        style: 0x1423300,
        tag: 2,
        reports: true,
        ..record(0x1, 0x0, 0x8008, 0x8000, 0x24, 0x0)
    },
    FormRecord {
        group: 32,
        text: Some("2"),
        style: 0x1423300,
        tag: 1,
        reports: true,
        ..record(0x1, 0x0, 0x8008, 0x8000, 0x24, 0x0)
    },
    FormRecord {
        group: 32,
        text: Some("1"),
        style: 0x1423300,
        state: 0x60,
        reports: true,
        ..record(0x1, 0x0, 0x8008, 0x8000, 0x24, 0x0)
    },
    FormRecord {
        text: Some("CLOSE"),
        style: 0x20483300,
        reports: true,
        ..record(0x1, 0x0, 0x24, 0x0, 0xa4, 0x1c)
    },
    FormRecord {
        text: Some("HIRE"),
        style: 0x204a3300,
        state: 0x110,
        reports: true,
        ..record(0x1, 0x0, 0x28, 0x0, 0xa4, 0x1c)
    },
    FormRecord {
        text: Some("Hire Fee:"),
        style: 0xa2300,
        state: 0x110,
        ..record(0x5, 0x0, 0x6c, 0xd, 0x60, 0x0)
    },
    FormRecord {
        style: 0xa0000,
        state: 0x110,
        amount: 0x400,
        ..record(0xac03, 0x0, 0x48, 0xd, 0x30, 0x0)
    },
    FormRecord {
        text: Some("Upkeep:"),
        style: 0xa2300,
        state: 0x110,
        ..record(0x5, 0x0, 0x6c, 0x8, 0x60, 0x0)
    },
    FormRecord {
        style: 0xa0000,
        state: 0x110,
        amount: 0x1,
        ..record(0xac03, 0x0, 0x48, 0x8, 0x30, 0x0)
    },
    FormRecord {
        text: Some("Profession: Combatant"),
        state: 0x110,
        ..record(0x5, 0x0, 0x8, 0x1e, 0xe4, 0x1c)
    },
];

/// The dismiss dialog's records: the prompt, Dismiss and Cancel.
pub const DISMISS_RECORDS: [FormRecord; 3] = [
    FormRecord {
        text: Some("Are you sure you want to dismiss "),
        style: 0x3b00,
        ..record(0x5, 0x0, 0x4, 0x4, 0x160, 0x0)
    },
    FormRecord {
        text: Some("Dismiss"),
        reports: true,
        ..record(0x1, 0x0, 0x68, 0x8005, 0x4c, 0x0)
    },
    FormRecord {
        text: Some("Cancel"),
        reports: true,
        ..record(0x1, 0x0, 0x8006, 0x8000, 0x4c, 0x0)
    },
];

/// The yes/no dialog's records: the prompt (set by the caller), Yes and No.
pub const YES_NO_RECORDS: [FormRecord; 3] = [
    FormRecord {
        style: 0x3b00,
        ..record(0x5, 0x0, 0x4, 0x4, 0x160, 0x0)
    },
    FormRecord {
        text: Some("Yes"),
        reports: true,
        ..record(0x1, 0x0, 0x68, 0x8005, 0x4c, 0x0)
    },
    FormRecord {
        text: Some("No"),
        reports: true,
        ..record(0x1, 0x0, 0x8006, 0x8000, 0x4c, 0x0)
    },
];

/// Manage Company's records by role.
pub mod manage {
    pub const ROSTER_TITLE: usize = 0;
    pub const REPORT_TITLE: usize = 1;
    pub const MANAGE: usize = 2;
    pub const ARSENAL: usize = 3;
    pub const RECRUIT: usize = 4;
    pub const HIRE: usize = 5;
    pub const OWNED: usize = 7;
    pub const UPKEEP: usize = 9;
    pub const EARNED: usize = 11;
    pub const SPENT: usize = 13;
    pub const END_DAY: usize = 14;
    pub const CLOSE: usize = 15;
    pub const RECORDS: usize = 16;
    pub const STATUS: usize = 17;
}

/// Company Records' records by role.
pub mod records {
    /// The value records of the eleven counted records, in the order of
    /// the labels (each label's record is one before).
    pub const COUNTS: [usize; 11] = [1, 3, 5, 7, 9, 11, 13, 15, 17, 19, 21];
    pub const EARNED: usize = 23;
    pub const SPENT: usize = 25;
    pub const CLOSE: usize = 26;
}

/// Company Status's records by role.
pub mod status {
    pub const NAME: usize = 0;
    pub const RENOWN: usize = 2;
    pub const REPUTATION: usize = 4;
    pub const MORALE: usize = 6;
    pub const STAFF: usize = 8;
    pub const MASTERS: usize = 10;
    pub const CLOSE: usize = 11;
}

/// The loadout form's records by role.
pub mod loadout {
    pub const TITLE: usize = 0;
    pub const LIST: usize = 1;
    pub const PREVIOUS: usize = 2;
    pub const NEXT: usize = 3;
    pub const DISMISS: usize = 4;
    pub const RANK_LABEL: usize = 5;
    pub const RANK: usize = 6;
    pub const POINTS: usize = 7;
    pub const ENCUMBRANCE_LABEL: usize = 8;
    pub const ENCUMBRANCE: usize = 9;
    pub const PANEL: usize = 10;
    /// The five shelf buttons (their callback values 4 down to 0).
    pub const SHELVES: [usize; 5] = [11, 12, 13, 14, 15];
    /// The five box buttons, '5' down to '1' (values 4 down to 0).
    pub const BOXES: [usize; 5] = [16, 17, 18, 19, 20];
    pub const CLOSE: usize = 21;
    pub const HIRE: usize = 22;
    pub const FEE_LABEL: usize = 23;
    pub const FEE: usize = 24;
    pub const UPKEEP_LABEL: usize = 25;
    pub const UPKEEP: usize = 26;
    pub const PROFESSION: usize = 27;
}

/// A dialog's records by role.
pub mod dialog {
    pub const PROMPT: usize = 0;
    pub const YES: usize = 1;
    pub const NO: usize = 2;
}

/// The placement word the company windows and the dialogs OR in: centred
/// across (0x800) and down (0x8000), with 0x3000.
pub const PLACEMENT: u32 = 0xb800;
/// The base form constructor's attribute word.
pub const BASE_ATTRIBUTES: u32 = 0x3_0022;
/// The upper-layer bit Company Records, Company Status and the dialogs set.
pub const UPPER_LAYER: u32 = 0x1000;
/// The base form constructor's border.
pub const BASE_BORDER: i32 = 2;

/// Manage Company's two tables, client-local at unit 4: the roster and
/// the report.
pub const MANAGE_TABLES: [Rect; 2] = [
    Rect {
        x: 0,
        y: 0x18,
        w: 0x158,
        h: 0x128,
    },
    Rect {
        x: 0x15c,
        y: 0x18,
        w: 0x158,
        h: 0x128,
    },
];

/// Manage Company.
pub const MANAGE_FORM: FormSpec = FormSpec {
    size: (0x2c4, 0x1d4),
    border: BASE_BORDER,
    placement: PLACEMENT,
    attributes: BASE_ATTRIBUTES,
    title: "Manage Company",
    records: &MANAGE_RECORDS,
    children: &MANAGE_TABLES,
};

/// Company Records.
pub const RECORDS_FORM: FormSpec = FormSpec {
    size: (0x164, 0x1d4),
    border: BASE_BORDER,
    placement: PLACEMENT,
    attributes: BASE_ATTRIBUTES | UPPER_LAYER,
    title: "Company Records",
    records: &RECORDS_RECORDS,
    children: &[],
};

/// Company Status.
pub const STATUS_FORM: FormSpec = FormSpec {
    size: (0x164, 0x1ec),
    border: BASE_BORDER,
    placement: PLACEMENT,
    attributes: BASE_ATTRIBUTES | UPPER_LAYER,
    title: "Company Status",
    records: &STATUS_RECORDS,
    children: &[],
};

/// The loadout form: 1024 × 768 at the base place, border 0, its own
/// attribute word and no placement bits.
pub const LOADOUT_FORM: FormSpec = FormSpec {
    size: (crate::loadout::FORM_WIDTH, crate::loadout::FORM_HEIGHT),
    border: 0,
    placement: 0,
    attributes: crate::loadout::FORM_FLAGS,
    title: "",
    records: &LOADOUT_RECORDS,
    children: &[],
};

/// The dismiss dialog.
pub const DISMISS_FORM: FormSpec = FormSpec {
    size: (0x188, 0x70),
    border: 0,
    placement: PLACEMENT,
    attributes: BASE_ATTRIBUTES | UPPER_LAYER,
    title: "Dismiss",
    records: &DISMISS_RECORDS,
    children: &[],
};

/// The yes/no dialog.
pub const YES_NO_FORM: FormSpec = FormSpec {
    size: (0x188, 0x70),
    border: 0,
    placement: PLACEMENT,
    attributes: BASE_ATTRIBUTES | UPPER_LAYER,
    title: "",
    records: &YES_NO_RECORDS,
    children: &[],
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record_form::RecordForm;

    fn place(form: &RecordForm, i: usize) -> (i32, i32, i32, i32) {
        let c = &form.controls()[i];
        (c.x, c.y, c.w, c.h)
    }

    #[test]
    fn the_tables_keep_their_counts_and_words() {
        assert_eq!(MANAGE_RECORDS.len(), 18);
        assert_eq!(RECORDS_RECORDS.len(), 27);
        assert_eq!(STATUS_RECORDS.len(), 12);
        assert_eq!(LOADOUT_RECORDS.len(), 28);
        assert_eq!(MANAGE_RECORDS[manage::STATUS].text, Some("Status"));
        assert_eq!(MANAGE_RECORDS[manage::END_DAY].state, 0x10);
        assert_eq!(MANAGE_RECORDS[manage::OWNED].class, 0xac03);
        assert_eq!(RECORDS_RECORDS[records::CLOSE].text, Some("CLOSE"));
        assert_eq!(RECORDS_RECORDS[records::SPENT].class, 0xac03);
        assert_eq!(STATUS_RECORDS[status::MORALE].text, Some("Good"));
        assert_eq!(LOADOUT_RECORDS[loadout::RANK].steps, 5);
        assert_eq!(LOADOUT_RECORDS[loadout::LIST].class, 0x12);
        assert_eq!(LOADOUT_RECORDS[loadout::SHELVES[0]].tag, 4);
        assert_eq!(LOADOUT_RECORDS[loadout::BOXES[4]].state, 0x60);
        assert_eq!(LOADOUT_RECORDS[loadout::PROFESSION].state, 0x110);
        assert_eq!(DISMISS_RECORDS[dialog::YES].text, Some("Dismiss"));
        assert_eq!(YES_NO_RECORDS[dialog::PROMPT].text, None);
        // Every button with a callback reports.
        assert!(MANAGE_RECORDS[2..6].iter().all(|r| r.reports));
        assert!(!MANAGE_RECORDS[manage::ROSTER_TITLE].reports);
    }

    #[test]
    fn manage_company_lays_out_at_unit_four() {
        let form = RecordForm::open(&MANAGE_FORM, (1280, 720), 4);
        assert_eq!(
            form.rect(),
            Rect {
                x: 286,
                y: 126,
                w: 708,
                h: 468
            }
        );
        // Manage on grid row 84 (336 px), less half of the 28-pixel button
        // over 16 (6); Arsenal on its row, its x 178 snapped to the grid.
        assert_eq!(place(&form, manage::MANAGE), (40, 330, 120, 28));
        assert_eq!(place(&form, manage::ARSENAL), (176, 330, 120, 28));
        // Recruit and Hire ten rows under them (the 0x8005 anchor).
        assert_eq!(place(&form, manage::RECRUIT), (40, 370, 120, 28));
        // The bottom row counts up from the client's bottom.
        let end = place(&form, manage::END_DAY);
        let close = place(&form, manage::CLOSE);
        assert_eq!(end.1, close.1);
        assert_eq!(close.0, end.0 + end.2 + 8);
        assert!(end.1 > 400);
        // The coin controls follow their labels on grid row 83: the label
        // has no width (its text is right-aligned against its x), the coin
        // starts there (the gap-0 anchor), 48 wide and 24 tall.
        let owned = place(&form, manage::OWNED);
        assert_eq!(place(&form, 6), (408, 330, 0, 20));
        assert_eq!(owned, (408, 328, 48, 24));
        // The tables are children of the client.
        assert_eq!(
            form.child(1),
            Some(Rect {
                x: 288 + 0x15c,
                y: 128 + 0x18,
                w: 0x158,
                h: 0x128
            })
        );
    }

    #[test]
    fn the_forms_rescale_with_the_unit() {
        let small = RecordForm::open(&RECORDS_FORM, (1920, 1080), 4);
        let big = RecordForm::open(&RECORDS_FORM, (1920, 1080), 8);
        // The client doubles; a border under 4 stays as it is.
        assert_eq!(big.rect().w, 2 * (small.rect().w - 4) + 4);
        let (a, b) = (place(&small, 0), place(&big, 0));
        assert_eq!((b.0, b.1), (2 * a.0, 2 * a.1));
        // The dialog keeps no border: its client is the whole form.
        let dialog = RecordForm::open(&DISMISS_FORM, (1280, 720), 4);
        assert_eq!(dialog.origin(), (dialog.rect().x, dialog.rect().y));
        assert_eq!(place(&dialog, dialog::YES).0, 0x68);
        assert_eq!(place(&dialog, dialog::NO).0, 0x68 + 0x4c + 0x10);
    }

    #[test]
    fn a_press_and_release_on_a_button_reports_it() {
        let mut form = RecordForm::open(&STATUS_FORM, (1280, 720), 4);
        let (ox, oy) = form.origin();
        let c = form.controls()[status::CLOSE].clone();
        let (x, y) = (ox + c.x + 4, oy + c.y + 4);
        assert_eq!(form.hit(x, y), Some(status::CLOSE));
        assert!(form.press(x, y).is_empty());
        assert_eq!(form.release(x, y), vec![status::CLOSE]);
        // A label without a callback lets the pointer through.
        let name = form.controls()[status::NAME].clone();
        assert_eq!(form.hit(ox + name.x + 2, oy + name.y + 2), None);
    }

    #[test]
    fn the_loadout_slider_steps_by_fifths() {
        let form = RecordForm::open(&LOADOUT_FORM, (1024, 768), 4);
        let rank = &form.controls()[loadout::RANK];
        assert_eq!(rank.steps, 5);
        assert_eq!(
            form.rect(),
            Rect {
                x: 0,
                y: 0,
                w: 1024,
                h: 768
            }
        );
        // HIRE and its fee stay hidden on the roster page.
        assert_eq!(form.controls()[loadout::HIRE].state, 0x110);
    }
}
