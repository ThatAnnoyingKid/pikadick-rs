use std::num::NonZeroU64;

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
    pub tags: Box<[Tag]>,
}

/// A Tag
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Tag {
    /// The tag kind.
    #[serde(rename = "@type")]
    pub kind: TagKind,

    /// The # of posts with this tag.
    ///
    /// This is not always up to date.
    #[serde(rename = "@count")]
    pub count: u64,

    /// The tag name.
    #[serde(rename = "@name")]
    pub name: Box<str>,

    /// ?
    #[serde(rename = "@ambiguous")]
    pub ambiguous: bool,

    /// The tag id.
    #[serde(rename = "@id")]
    pub id: NonZeroU64,
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
        match *self {
            Self::GENERAL => "General".fmt(f),
            Self::AUTHOR => "Author".fmt(f),
            Self::COPYRIGHT => "Copyright".fmt(f),
            Self::CHARACTER => "Character".fmt(f),
            Self::METADATA => "Metadata".fmt(f),
            Self(unknown) => write!(f, "Unknown({unknown})"),
        }
    }
}
