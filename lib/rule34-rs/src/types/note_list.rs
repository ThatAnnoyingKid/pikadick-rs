use std::num::NonZeroU64;
use time::OffsetDateTime;

/// A list of notes for posts
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct NoteList {
    /// The list of notes
    #[serde(rename = "note", default)]
    pub notes: Box<[Note]>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Note {
    /// The id of the note.
    ///
    /// Together with the version, this creates a unique id for the note.
    #[serde(rename = "@id")]
    pub id: u64,

    /// The version of the note.
    ///
    /// Together with the id, this creates a unique id for the note.
    #[serde(rename = "@version")]
    pub version: NonZeroU64,

    /// The time of the last update.
    #[serde(rename = "@updated_at", with = "crate::util::asctime_with_offset")]
    pub updated_at: OffsetDateTime,

    /// ?
    #[serde(rename = "@is_active")]
    pub is_active: bool,

    /// The time of the creation.
    #[serde(rename = "@created_at", with = "crate::util::asctime_with_offset")]
    pub created_at: OffsetDateTime,

    /// The x position.
    #[serde(rename = "@x")]
    pub x: u64,

    /// The y position
    #[serde(rename = "@y")]
    pub y: u64,

    /// The width
    #[serde(rename = "@width")]
    pub width: u64,

    /// The height
    #[serde(rename = "@height")]
    pub height: u64,

    /// The note body
    #[serde(rename = "@body")]
    pub body: Box<str>,

    /// The creator
    #[serde(rename = "@creator_id")]
    pub creator_id: NonZeroU64,
}
