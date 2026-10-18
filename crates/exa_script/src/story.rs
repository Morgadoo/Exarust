//! Story progression, a conversation's choices, and the journal.
//!
//! The campaign named here is `exanima01` through `exanima08`. The side
//! levels (catacombs, sewers, gardens) are not in this chain. A topic can
//! be chosen once; the chosen line is the reply. The journal starts as the
//! notebook's first entry, titled "New Entry", and stores the text written
//! into the open entry.
//!
//! Evidence: the story level ids, the dialogue choice list (a pick runs
//! that line and topic memory keeps it from being offered again), and the
//! journal window (first entry "New Entry", close stores the edited text).

/// The eight story levels, in campaign order.
pub const CAMPAIGN: [&str; 8] = [
    "exanima01",
    "exanima02",
    "exanima03",
    "exanima04",
    "exanima05",
    "exanima06",
    "exanima07",
    "exanima08",
];

/// One page of the journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub title: String,
    pub text: String,
}

/// The notebook. It always has a current entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Journal {
    entries: Vec<Entry>,
    current: usize,
}

impl Journal {
    /// The notebook as it opens: one blank page titled "New Entry".
    pub fn new() -> Self {
        Self {
            entries: vec![Entry {
                title: "New Entry".to_string(),
                text: String::new(),
            }],
            current: 0,
        }
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Store text into the open page.
    pub fn write(&mut self, text: &str) {
        self.entries[self.current].text = text.to_string();
    }

    /// Open a new titled page.
    pub fn add(&mut self, title: &str) {
        self.entries.push(Entry {
            title: title.to_string(),
            text: String::new(),
        });
        self.current = self.entries.len() - 1;
    }

    pub fn from_entries(entries: Vec<Entry>) -> Option<Self> {
        if entries.is_empty() {
            return None;
        }
        Some(Self {
            current: entries.len() - 1,
            entries,
        })
    }
}

impl Default for Journal {
    fn default() -> Self {
        Self::new()
    }
}

/// A conversation topic and the replies the player can pick.
#[derive(Debug, Clone, Copy)]
pub struct Topic<'a> {
    pub title: &'a str,
    pub choices: &'a [&'a str],
}

/// The story session: which level is loaded, the journal, and used topics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Story {
    index: usize,
    journal: Journal,
    used: Vec<String>,
}

impl Story {
    /// A new story, standing in `exanima01`.
    pub fn begin() -> Self {
        Self {
            index: 0,
            journal: Journal::new(),
            used: Vec::new(),
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    /// The level that is loaded.
    pub fn level(&self) -> &'static str {
        CAMPAIGN[self.index]
    }

    /// Load the next level. The last level stays loaded.
    pub fn advance(&mut self) -> &'static str {
        if self.index + 1 < CAMPAIGN.len() {
            self.index += 1;
        }
        self.level()
    }

    pub fn journal(&self) -> &Journal {
        &self.journal
    }

    pub fn journal_mut(&mut self) -> &mut Journal {
        &mut self.journal
    }

    pub fn topics_used(&self) -> &[String] {
        &self.used
    }

    /// Whether this topic is still offered.
    pub fn available(&self, topic: Topic<'_>) -> bool {
        !self.used.iter().any(|title| title == topic.title)
    }

    /// Pick a reply. The topic is not offered again.
    pub fn pick<'a>(&mut self, topic: Topic<'a>, choice: usize) -> Result<&'a str, TalkError> {
        if !self.available(topic) {
            return Err(TalkError::Used);
        }
        let line = topic
            .choices
            .get(choice)
            .copied()
            .ok_or(TalkError::NoSuchChoice)?;
        self.used.push(topic.title.to_string());
        Ok(line)
    }

    /// Rebuild a story the save read. An index past the campaign is refused.
    pub fn restore(index: usize, journal: Journal, used: Vec<String>) -> Option<Self> {
        if index >= CAMPAIGN.len() {
            return None;
        }
        Some(Self {
            index,
            journal,
            used,
        })
    }
}

impl Default for Story {
    fn default() -> Self {
        Self::begin()
    }
}

/// Why a reply was not taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TalkError {
    /// That topic was already chosen.
    Used,
    /// The choice number is not on the topic.
    NoSuchChoice,
}

#[cfg(test)]
mod tests {
    use super::*;

    const GREETING: Topic<'static> = Topic {
        title: "The gate",
        choices: &["I go on.", "I turn back."],
    };

    #[test]
    fn the_story_advances_and_a_choice_is_kept() {
        let mut story = Story::begin();
        assert_eq!(story.level(), "exanima01");
        assert_eq!(story.journal().entries()[0].title, "New Entry");
        assert!(story.journal().entries()[0].text.is_empty());
        story.journal_mut().write("The pit is open.");
        assert_eq!(story.journal().entries()[0].text, "The pit is open.");
        for (step, name) in CAMPAIGN.iter().enumerate().skip(1) {
            assert_eq!(story.advance(), *name, "step {step}");
        }
        assert_eq!(story.advance(), "exanima08");
        assert!(story.available(GREETING));
        assert_eq!(story.pick(GREETING, 1).unwrap(), "I turn back.");
        assert!(!story.available(GREETING));
        assert_eq!(story.pick(GREETING, 0), Err(TalkError::Used));
        assert_eq!(story.pick(GREETING, 9), Err(TalkError::Used));
    }
}
