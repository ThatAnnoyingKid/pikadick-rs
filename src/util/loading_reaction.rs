use serenity::{
    http::Http,
    model::prelude::*,
};
use std::sync::Arc;

const LOADING_EMOJI: char = '⌛';
const OK_EMOJI: char = '✅';
const ERR_EMOJI: char = '❌';

/// This type attaches to a message and displays a loading sign until `send_ok` or `send_err` are called,
/// where it then displays a check or an X respectively.
/// If neither are called, send_err is called automatically from the destructor.
/// All functions are not async and can only be used from a tokio runtime context.
/// Errors are silently ignored.
pub struct LoadingReaction {
    http: Arc<Http>,
    channel_id: ChannelId,
    msg_id: MessageId,

    sent_reaction: bool,
}

impl std::fmt::Debug for LoadingReaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadingReaction")
            .field("channel_id", &self.channel_id)
            .field("msg_id", &self.msg_id)
            .field("sent_reaction", &self.sent_reaction)
            .finish()
    }
}

impl LoadingReaction {
    /// Create a Loading Reaction attatched to a message.
    pub fn new(http: Arc<Http>, msg: &Message) -> Self {
        let channel_id = msg.channel_id;
        let msg_id = msg.id;

        let ret = LoadingReaction {
            http,
            channel_id,
            msg_id,

            sent_reaction: false,
        };

        ret.send_reaction(LOADING_EMOJI);

        ret
    }

    pub fn send_reaction<T: Into<ReactionType>>(&self, reaction: T) {
        {
            let msg_id = self.msg_id;
            let channel_id = self.channel_id;
            let http = self.http.clone();
            let reaction = reaction.into();

            tokio::spawn(async move {
                http.create_reaction(channel_id.0, msg_id.0, &reaction)
                    .await
                    .ok();
            });
        }
    }

    pub fn send_ok(&mut self) {
        self.send_reaction(OK_EMOJI);
        self.sent_reaction = true;
    }

    pub fn send_fail(&mut self) {
        self.send_reaction(ERR_EMOJI);
        self.sent_reaction = true;
    }
}

impl Drop for LoadingReaction {
    fn drop(&mut self) {
        if !self.sent_reaction {
            self.send_fail();
        }
    }
}
