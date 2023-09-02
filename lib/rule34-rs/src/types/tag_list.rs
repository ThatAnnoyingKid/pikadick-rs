/// A list of tags
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct TagList {
    /// The tag list kind?
    ///
    /// So far, this has only been "array"
    #[serde(rename = "type", alias = "@type")]
    pub kind: Box<str>,

    /// The list of tags
    #[serde(rename = "tag", default)]
    pub tags: Vec<Tag>,
}

/// A Tag
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Tag {
    /// The tag kind
    #[serde(rename = "type", alias = "@type")]
    pub kind: TagKind,

    /// The # of posts with this tag?
    #[serde(alias = "@count")]
    pub count: u64,

    /// The tag name
    #[serde(alias = "@name")]
    pub name: Box<str>,

    /// ?
    #[serde(alias = "@ambiguous")]
    pub ambiguous: bool,

    /// The tag id
    #[serde(alias = "@id")]
    pub id: u64,
}

/// The tag kind
#[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct TagKind(pub u64);

impl TagKind {
    // When adding more variants, make sure to update the Debug impl and and functions to test for it
    pub const GENERAL: Self = TagKind(0);
    pub const AUTHOR: Self = TagKind(1);
    pub const COPYRIGHT: Self = TagKind(3);
    pub const CHARACTER: Self = TagKind(4);
    pub const METADATA: Self = TagKind(5);

    /// Returns true if this tag kind is general.
    pub fn is_general(self) -> bool {
        self == Self::GENERAL
    }

    /// Returns true if this tag kind is an author.
    pub fn is_author(self) -> bool {
        self == Self::AUTHOR
    }

    /// Retruns true if this tag is a copyright.
    pub fn is_copyright(self) -> bool {
        self == Self::COPYRIGHT
    }

    /// Returns true if this tag kind is a character.
    pub fn is_character(self) -> bool {
        self == Self::CHARACTER
    }

    /// Returns true if this tag kind is a metadata.
    pub fn is_metadata(self) -> bool {
        self == Self::METADATA
    }

    /// Returns true if the tag kind is unknown.
    pub fn is_unknown(self) -> bool {
        !self.is_general()
            && !self.is_author()
            && !self.is_copyright()
            && !self.is_character()
            && !self.is_metadata()
    }
}

impl std::fmt::Debug for TagKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.is_general() {
            "General".fmt(f)
        } else if self.is_author() {
            "Author".fmt(f)
        } else if self.is_copyright() {
            "Copyright".fmt(f)
        } else if self.is_character() {
            "Character".fmt(f)
        } else if self.is_metadata() {
            "Metadata".fmt(f)
        } else {
            write!(f, "Unknown({})", self.0)
        }
    }
}
