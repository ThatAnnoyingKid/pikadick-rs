use crate::checks::ENABLED_CHECK;
use serenity::{
    client::Context,
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::channel::Message,
};
use std::collections::HashMap;
use unicase::UniCase;

lazy_static::lazy_static! {
    static ref KNOWN_MAP: HashMap<UniCase<&'static str>, &'static str> = {
          let mut map = HashMap::new();
          {
              let response = "wrong but go off";
              map.insert("you are wrong".into(), response);
              map.insert("your wrong".into(), response);
              map.insert("you're wrong".into(), response);
          }
          {
            let response = "PERIOD QUEEN 👑";
            map.insert("i agree".into(), response);
          }

          map
    };
}

#[command]
#[description("Twitterify a phrase")]
#[usage("\"<phrase>\"")]
#[example("\"Hello World!\"")]
#[min_args(1)]
#[max_args(1)]
#[checks(Enabled)]
pub async fn twitterify(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let phrase = args.single_quoted::<String>()?;
    msg.channel_id
        .say(&ctx.http, twitterify_str(&phrase))
        .await?;
    Ok(())
}

/// Twitterify the input
pub fn twitterify_str(input: &str) -> String {
    let mut ret = String::with_capacity(input.len());

    // Known translation
    if let Some(response) = KNOWN_MAP.get(&input.into()) {
        return response.to_string();
    }

    for word in input.split(' ') {
        // TODO: Consider trie
        if word == "sweetie" {
            ret.push_str("sweaty");
        } else if word == "sister" {
            ret.push_str("sis");
        } else {
            ret.push_str(word);
        }

        ret.push(' ');
    }

    // Endings
    if input.ends_with('.') {
        ret.push_str("PERIODOT");
    } else {
        ret.push('💅'); // TODO: Randomize colors
    }

    ret
}
