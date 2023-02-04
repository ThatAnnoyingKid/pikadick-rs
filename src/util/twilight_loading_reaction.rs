use anyhow::Context;
use tracing::warn;
use twilight_http::{
    client::Client,
    request::channel::reaction::RequestReactionType,
};
use twilight_model::id::{
    marker::{
        ChannelMarker,
        MessageMarker,
    },
    Id,
};

const LOADING_EMOJI: &str = "⌛";
const OK_EMOJI: &str = "✅";
const ERR_EMOJI: &str = "❌";

/// A loading reaction based on twilight.
///
/// This type attaches to a message and displays a loading sign until `send_ok` or `send_err` are called,
/// where it then displays a check or an X respectively.
///
/// If neither are called, send_err is called automatically from the destructor.
/// All functions are NOT async, but they can only be used from a tokio runtime context.
///
/// Errors are silently ignored.
pub struct TwilightLoadingReaction<C>
where
    C: CloneClient,
{
    client: C,
    channel_id: Id<ChannelMarker>,
    message_id: Id<MessageMarker>,

    sent_reaction: bool,
}

impl<C> TwilightLoadingReaction<C>
where
    C: CloneClient,
{
    /// Create a Loading Reaction attatched to a message.
    pub fn new(client: C, channel_id: Id<ChannelMarker>, message_id: Id<MessageMarker>) -> Self {
        let ret = Self {
            client,
            channel_id,
            message_id,

            sent_reaction: false,
        };

        ret.send_reaction(RequestReactionType::Unicode {
            name: LOADING_EMOJI,
        });

        ret
    }

    /// Send a reaction.
    fn send_reaction(&self, reaction: RequestReactionType<'static>) {
        let message_id = self.message_id;
        let channel_id = self.channel_id;
        let client = self.client.clone();

        tokio::spawn(async move {
            if let Err(e) = client
                .client()
                .create_reaction(channel_id, message_id, &reaction)
                .exec()
                .await
                .context("failed to create reaction")
            {
                warn!("{e:?}");
            }
        });
    }

    /// Send the `Ok` reaction.
    pub fn send_ok(&mut self) {
        self.send_reaction(RequestReactionType::Unicode { name: OK_EMOJI });
        self.sent_reaction = true;
    }

    /// Send the `Fail` reaction.
    pub fn send_fail(&mut self) {
        self.send_reaction(RequestReactionType::Unicode { name: ERR_EMOJI });
        self.sent_reaction = true;
    }
}

impl<C> std::fmt::Debug for TwilightLoadingReaction<C>
where
    C: CloneClient,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TwilightLoadingReaction")
            .field("channel_id", &self.channel_id)
            .field("message_id", &self.message_id)
            .field("sent_reaction", &self.sent_reaction)
            .finish()
    }
}

impl<C> Drop for TwilightLoadingReaction<C>
where
    C: CloneClient,
{
    fn drop(&mut self) {
        if !self.sent_reaction {
            self.send_fail();
        }
    }
}

/// A trait wrapper for a twilight Client that requires implementors to somehow make it clonable.
pub trait CloneClient: Clone + Send + Sync + 'static {
    fn client(&self) -> &Client;
}
