use crate::{
    types::{
        CheckRoomJsonRequest,
        GenericResponse,
    },
    Error,
};

const CHECK_ROOM_URI: &str = "https://game.quizizz.com/play-api/v5/checkRoom";

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
    pub async fn check_room(&self, room_code: &str) -> Result<GenericResponse, Error> {
        Ok(self
            .client
            .post(CHECK_ROOM_URI)
            .json(&CheckRoomJsonRequest { room_code })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
