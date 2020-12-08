use crate::{
    types::{
        CheckRoomJsonRequest,
        GenericResponse,
    },
    QError,
    QResult,
};

const CHECK_ROOM_URI: &str = "https://game.quizizz.com/play-api/v4/checkRoom";

/// A quizizz Client
#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
}

impl Client {
    /// Make a new client
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Check if the room code exists
    pub async fn check_room(&self, room_code: &'_ str) -> QResult<GenericResponse> {
        let res = self
            .client
            .post(CHECK_ROOM_URI)
            .json(&CheckRoomJsonRequest { room_code })
            .send()
            .await?;

        let status = res.status();
        if !status.is_success() {
            return Err(QError::InvalidStatus(status));
        }

        Ok(res.json().await?)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
