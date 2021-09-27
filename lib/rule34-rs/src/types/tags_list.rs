/// A list of tags
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct TagsList {
    /// The tag list kind?
    ///
    /// So far, this has only been "array"
    #[serde(rename = "type")]
    pub kind: String,

    /// The list of tags
    #[serde(rename = "tag", default)]
    pub tags: Vec<Tag>,
}

/// A Tag
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Tag {
    /// The tag kind
    #[serde(rename = "type")]
    pub kind: TagKind,

    /// The # of posts with this tag?
    pub count: u64,

    /// The tag name
    pub name: String,

    /// ?
    pub ambiguous: bool,

    /// The tag id
    pub id: u64,
}

// When adding more variants, make sure to update the Debug impl and and functions to test for it
pub const TAG_KIND_GENERAL: TagKind = TagKind(0);
pub const TAG_KIND_AUTHOR: TagKind = TagKind(1);
pub const TAG_KIND_COPYRIGHT: TagKind = TagKind(3);
pub const TAG_KIND_CHARACTER: TagKind = TagKind(4);

/// The tag kind
#[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct TagKind(pub u64);

impl TagKind {
    /// Returns true if this tag kind is general
    pub fn is_general(self) -> bool {
        self == TAG_KIND_GENERAL
    }

    /// Returns true if this tag kind is an author
    pub fn is_author(self) -> bool {
        self == TAG_KIND_AUTHOR
    }

    /// Retruns true if this tag is a copyright
    pub fn is_copyright(self) -> bool {
        self == TAG_KIND_COPYRIGHT
    }

    /// Returns true if this tag kind is a character
    pub fn is_character(self) -> bool {
        self == TAG_KIND_CHARACTER
    }

    /// Returns true if the tag kind is unknown
    pub fn is_unknown(self) -> bool {
        !self.is_author() && !self.is_character() && !self.is_copyright()
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
        } else {
            write!(f, "Unknown({})", self.0)
        }
    }
}
