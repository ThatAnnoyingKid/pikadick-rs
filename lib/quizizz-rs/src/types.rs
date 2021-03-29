use std::collections::HashMap;

/// Check Room Request
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct CheckRoomJsonRequest<'a> {
    /// Room code
    #[serde(rename = "roomCode")]
    pub room_code: &'a str,
}

/// Api Response
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GenericResponse {
    /// Unknown
    #[serde(rename = "__cid__")]
    pub cid: serde_json::Value,

    /// Error
    pub error: Option<GenericResponseError>,

    /// Room Object
    pub room: Option<Room>,

    /// Player Object
    pub player: Option<serde_json::Value>,

    /// Other
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Api Response Error
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GenericResponseError {
    /// Error Message
    pub message: String,

    /// Error Type
    #[serde(rename = "type")]
    pub kind: String,

    /// Other
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A quiz Room
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Room {
    /// Total # of questions
    #[serde(rename = "totalQuestions")]
    pub total_questions: Option<i64>,

    /// List of question ids
    pub questions: Option<Vec<String>>,

    /// Unknown
    pub assignments: Option<serde_json::Value>,

    /// Whether quiz was deleted
    pub deleted: Option<bool>,

    /// Creation date
    #[serde(rename = "createdAt")]
    pub created_at: u64,

    /// Unit ID
    #[serde(rename = "unitId")]
    pub unit_id: Option<serde_json::Value>,

    /// Quiz hash
    pub hash: String,

    /// Unknown
    #[serde(rename = "createGroup")]
    pub create_group: Option<serde_json::Value>,

    /// Unknown
    #[serde(rename = "replayOf")]
    pub replay_of: Option<serde_json::Value>,

    /// Whether the quiz is reopenable
    pub reopenable: Option<bool>,

    /// Quiz traits
    pub traits: Option<Traits>,

    /// Host Id
    #[serde(rename = "hostId")]
    pub host_id: Option<String>,

    /// Started at
    #[serde(rename = "startedAt")]
    pub started_at: Option<u64>,

    /// Host Session ID
    #[serde(rename = "hostSessionId")]
    pub host_session_id: Option<String>,

    /// Quiz ID
    #[serde(rename = "quizId")]
    pub quiz_id: Option<String>,

    /// Group Ids
    #[serde(rename = "groupIds")]
    pub group_ids: Option<Vec<serde_json::Value>>,

    /// Groups info
    #[serde(rename = "groupsInfo")]
    pub groups_info: Option<GroupsInfo>,

    /// Version Id
    #[serde(rename = "versionId")]
    pub version_id: Option<String>,

    /// Total players
    #[serde(rename = "totalPlayers")]
    pub total_players: Option<u64>,

    /// Total correct
    #[serde(rename = "totalCorrect")]
    pub total_correct: Option<u64>,

    /// Expire interval
    pub expiry: f64,

    /// Room options
    pub options: Option<RoomOptions>,

    /// Versions
    pub version: Vec<RoomVersion>,

    /// Whether this game is a simulation
    #[serde(rename = "simGame")]
    pub sim_game: Option<bool>,

    /// Course ID
    #[serde(rename = "courseId")]
    pub course_id: Option<serde_json::Value>,

    /// Assignment Title
    #[serde(rename = "assignmentTitle")]
    pub assignment_title: Option<serde_json::Value>,

    /// Whether this room is shared
    #[serde(rename = "isShared")]
    pub is_shared: Option<bool>,

    /// Room subscription info
    pub subscription: Option<serde_json::Value>,

    /// Whether this was reopened
    pub reopened: Option<bool>,

    /// The type of room
    #[serde(rename = "type")]
    pub kind: String,

    /// Unknown
    pub experiment: Option<String>,

    /// Unknown
    #[serde(rename = "collectionId")]
    pub collection_id: Option<serde_json::Value>,

    /// Host Occupation
    #[serde(rename = "hostOccupation")]
    pub host_occupation: Option<String>,

    /// Room state
    pub state: Option<String>,

    /// The number of answerable questions
    #[serde(rename = "totalAnswerableQuestions")]
    pub total_answerable_questions: Option<u64>,

    /// Organization
    pub organization: Option<String>,

    /// Room Code
    pub code: Option<String>,

    /// Unknown
    #[serde(rename = "soloApis")]
    pub solo_apis: Option<serde_json::Value>,

    /// Other
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl Room {
    /// Whether this room is running
    ///
    pub fn is_running(&self) -> bool {
        self.state.as_deref() == Some("running")
    }
}

/// Traits
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Traits {
    /// Whether questions don't have correct answers
    #[serde(rename = "isQuizWithoutCorrectAnswer")]
    pub is_quiz_without_correct_answer: bool,

    /// The # of slides
    #[serde(rename = "totalSlides")]
    pub total_slides: Option<u64>,

    /// Other
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Groups Info
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GroupsInfo {
    /// Unknown
    pub assigned: Vec<serde_json::Value>,

    /// Unknown
    #[serde(rename = "assignedTo")]
    pub assigned_to: serde_json::Value,

    /// Unknown
    pub create: serde_json::Value,

    /// Unknown
    pub data: GroupsInfoData,

    /// Unknown
    pub gcl: Vec<serde_json::Value>,

    /// Whether there is a gcl
    #[serde(rename = "hasGCL")]
    pub has_gcl: bool,

    /// Mode
    pub mode: String,

    /// Other
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Groups Info Data
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GroupsInfoData {
    /// Description
    pub description: Option<serde_json::Value>,

    /// Title
    pub title: Option<serde_json::Value>,

    /// Other
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Room Options
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct RoomOptions {
    /// Unknown
    pub jumble: bool,

    /// Unknown
    #[serde(rename = "jumbleAnswers")]
    pub jumble_answers: bool,

    /// Attempt Limit
    #[serde(rename = "limitAttempts")]
    pub limit_attempts: u64,

    /// Whether powerups are enabled
    pub powerups: String,

    /// Whether memes are enabled
    pub memes: bool,

    /// Whether answers are shown
    #[serde(rename = "showAnswers")]
    pub show_answers: bool,

    // No longer present?
    // /// Memeset hash
    // pub memeset: String,
    /// Show Answers as str
    #[serde(rename = "showAnswers_2")]
    pub show_answers_2: String,

    /// Whether it is a student review
    #[serde(rename = "studentQuizReview")]
    pub student_quiz_review: bool,

    /// Wtudent quiz review as str
    #[serde(rename = "studentQuizReview_2")]
    pub student_quiz_review_2: String,

    /// Whether there is a timer
    pub timer: bool,

    /// Whether there is music
    #[serde(rename = "studentMusic")]
    pub student_music: bool,

    /// Whether there is a student leaderboard
    #[serde(rename = "studentLeaderboard")]
    pub student_leaderboard: bool,

    /// Whether redemption is active
    pub redemption: String,

    /// Whether login is required
    #[serde(rename = "loginRequired")]
    pub login_required: bool,

    /// Whether nickname generator is used
    #[serde(rename = "nicknameGenerator")]
    pub nickname_generator: bool,

    /// Other
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Room Version Info
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct RoomVersion {
    /// Type
    #[serde(rename = "type")]
    pub kind: String,

    /// Version number
    pub version: u64,

    /// Other
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Room subscription info
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct RoomSubscription {
    /// Whether it is ads free
    #[serde(rename = "adsFree")]
    pub ads_free: bool,

    /// Unknown
    pub branding: bool,

    /// The player limit
    #[serde(rename = "playerLimit")]
    pub player_limit: u64,

    /// When the trial ends
    #[serde(rename = "trialEndAt")]
    pub trial_end_at: Option<serde_json::Value>,

    /// Other
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod test {
    use super::*;

    const ROOM_NOT_FOUND_GENERIC_RESPONSE_STR: &str =
        include_str!("../test_data/RoomNotFoundGenericResponse.json");
    const ROOM_FOUND_GENERIC_RESPONSE_1_STR: &str =
        include_str!("../test_data/RoomFoundGenericResponse1.json");
    const ROOM_FOUND_GENERIC_RESPONSE_2_STR: &str =
        include_str!("../test_data/RoomFoundGenericResponse2.json");
    const ROOM_FOUND_GENERIC_RESPONSE_3_STR: &str =
        include_str!("../test_data/RoomFoundGenericResponse3.json");
    const ROOM_FOUND_GENERIC_RESPONSE_4_STR: &str =
        include_str!("../test_data/RoomFoundGenericResponse4.json");

    #[test]
    fn room_not_found_generic_response() {
        let data: GenericResponse =
            serde_json::from_str(ROOM_NOT_FOUND_GENERIC_RESPONSE_STR).unwrap();
        dbg!(data);
    }

    #[test]
    fn room_found_generic_response_1() {
        let data: GenericResponse =
            serde_json::from_str(ROOM_FOUND_GENERIC_RESPONSE_1_STR).unwrap();
        dbg!(data);
    }

    #[test]
    fn room_found_generic_response_2() {
        let data: GenericResponse =
            serde_json::from_str(ROOM_FOUND_GENERIC_RESPONSE_2_STR).unwrap();
        dbg!(data);
    }

    #[test]
    fn room_found_generic_response_3() {
        let data: GenericResponse =
            serde_json::from_str(ROOM_FOUND_GENERIC_RESPONSE_3_STR).unwrap();
        dbg!(data);
    }

    #[test]
    fn room_found_generic_response_4() {
        let data: GenericResponse =
            serde_json::from_str(ROOM_FOUND_GENERIC_RESPONSE_4_STR).unwrap();
        dbg!(data);
    }
}
