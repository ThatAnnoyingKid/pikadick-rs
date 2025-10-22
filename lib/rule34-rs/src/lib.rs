mod client;
mod error;
mod search_query_builder;
mod types;
mod util;

#[cfg(feature = "scrape")]
pub use crate::types::HtmlPost;
pub use crate::{
    client::{
        Client,
        NotesListQueryBuilder,
        PostListQueryBuilder,
        TagListQueryBuilder,
    },
    error::Error,
    search_query_builder::SearchQueryBuilder,
    types::{
        DeletedImageList,
        Note,
        NoteList,
        Post,
        PostList,
        PostStatus,
        Rating,
        Tag,
        TagKind,
        TagList,
    },
};
#[cfg(feature = "scrape")]
pub use scraper::Html;
use std::num::NonZeroU64;
pub use url::Url;

/// The maximum number of responses per post list request
pub const POST_LIST_LIMIT_MAX: u16 = 1_000;
/// The maximum number of responses per tags list request.
///
/// This is undocumented.
/// The documented limit is 100.
pub const TAGS_LIST_LIMIT_MAX: u16 = 1_000;

// URL constants
pub(crate) const URL_INDEX: &str = "https://rule34.xxx/index.php";

/// The base Api Url
pub(crate) const API_BASE_URL: &str = "https://api.rule34.xxx/index.php";

/// Turn a post id into a post url
fn post_id_to_html_post_url(id: NonZeroU64) -> Url {
    // It shouldn't be possible to make this function fail for any valid id.
    Url::parse_with_params(
        crate::URL_INDEX,
        &[
            ("id", itoa::Buffer::new().format(id.get())),
            ("page", "post"),
            ("s", "view"),
        ],
    )
    .unwrap()
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::LazyLock;

    #[derive(serde::Deserialize)]
    struct Config {
        user_id: u64,
        api_key: String,
    }

    impl Config {
        fn load() -> Self {
            let raw =
                std::fs::read_to_string("./config.json").expect("failed to read \"config.json\"");
            let config: Config = serde_json::from_str(&raw).expect("failed to parse config");

            config
        }
    }

    static CONFIG: LazyLock<Config> = LazyLock::new(Config::load);

    static RUNTIME: LazyLock<tokio::runtime::Runtime> =
        LazyLock::new(|| tokio::runtime::Runtime::new().expect("failed to init runtime"));

    static CLIENT: LazyLock<Client> = LazyLock::new(|| {
        let client = Client::new();
        client.set_auth(CONFIG.user_id, &CONFIG.api_key);
        client
    });

    #[ignore]
    #[test]
    fn search() {
        let res = RUNTIME
            .block_on(CLIENT.list_posts().tags(Some("rust")).execute())
            .expect("failed to search rule34 for \"rust\"");
        dbg!(&res);
        assert!(!res.posts.is_empty());
    }

    async fn get_top_post(query: &str) {
        let response = CLIENT
            .list_posts()
            .tags(Some(query))
            .limit(Some(crate::POST_LIST_LIMIT_MAX))
            .execute()
            .await
            .unwrap_or_else(|error| panic!("failed to search rule34 for \"{query}\": {error}"));
        assert!(!response.posts.is_empty(), "no posts for \"{query}\"");

        dbg!(&response);

        #[cfg(feature = "scrape")]
        {
            let first = response.posts.first().expect("missing first entry");
            let post = CLIENT
                .get_html_post(first.id)
                .await
                .expect("failed to get first post");
            dbg!(post);
        }
    }

    #[ignore]
    #[test]
    fn it_works() {
        let list = [
            "rust",
            "fbi",
            "gif",
            "corna",
            "sledge",
            "roadhog",
            "deep_space_waifu",
            "aokuro",
        ];

        RUNTIME.block_on(async move {
            for item in list {
                get_top_post(item).await;
            }
        });
    }

    #[ignore]
    #[test]
    fn deleted_images_list() {
        // Just choose a high-ish post id here and update to keep the download limited
        let min = Some(NonZeroU64::new(826_550).unwrap());

        let result = RUNTIME
            .block_on(CLIENT.list_deleted_images(min))
            .expect("failed to get deleted images");
        dbg!(result);
    }

    #[ignore]
    #[test]
    fn tags_list() {
        let result = RUNTIME
            .block_on(
                CLIENT
                    .list_tags()
                    .limit(Some(crate::TAGS_LIST_LIMIT_MAX))
                    .order(Some("name"))
                    .execute(),
            )
            .expect("failed to list tags");
        assert!(!result.tags.is_empty());
        // dbg!(result);
    }

    #[ignore]
    #[test]
    fn bad_tags_list() {
        let tags = [
            // TODO: I think these tags were deleted
            // "swallow_(pokémon_move)",
            // "almáriel",
            "akoúo̱_(rwby)",
            "miló_(rwby)",
            "las_tres_niñas_(company)",
            "ooparts♥love",
            "kingdom_hearts_union_χ_[cross]",
            "gen¹³",
            "nancy’s_face_is_deeper_in_carrie’s_ass",
            "…",
            "cleaning_&_clearing_(blue_archive)",
            "watashi_ga_suki_nara_\"suki\"_tte_itte!",
            "<3",
            ">_<",
            "dr—worm",
            "master_hen'tai",
            "ne-α_parasite",
            "ne-α_type",
            "lützow_(azur_lane)",
            "ä",
            // "göll_(shuumatsu_no_valkyrie)",
        ];

        RUNTIME.block_on(async move {
            for expected_tag_name in tags {
                let tags = CLIENT
                    .list_tags()
                    .name(Some(expected_tag_name))
                    .execute()
                    .await
                    .expect("failed to get tag")
                    .tags;
                let tags_len = tags.len();

                assert!(
                    tags_len == 1,
                    "failed to get tags for \"{expected_tag_name}\", tags does not have one tag, it has {tags_len} tags"
                );
                let tag = tags.first().expect("tag list is empty");
                let actual_tag_name = &*tag.name;

                assert!(
                    actual_tag_name == expected_tag_name,
                    "\"{actual_tag_name}\" != \"{expected_tag_name}\""
                );
            }
        });
    }

    #[ignore]
    #[test]
    fn notes_list() {
        let result = RUNTIME
            .block_on(CLIENT.list_notes().execute())
            .expect("failed to list notes");
        assert!(!result.notes.is_empty());
        dbg!(result);
    }

    #[ignore]
    #[test]
    fn source() {
        let response_1 = RUNTIME
            .block_on(CLIENT.list_posts().id(NonZeroU64::new(1)).execute())
            .expect("failed to get post 1");
        let post_1 = response_1.posts.first().expect("missing post");
        assert!(post_1.id.get() == 1);
        assert!(post_1.source.is_none());

        let response_3 = RUNTIME
            .block_on(CLIENT.list_posts().id(NonZeroU64::new(3)).execute())
            .expect("failed to get post 3");
        let post_3 = response_3.posts.first().expect("missing post");
        assert!(post_3.id.get() == 3);
        assert!(post_3.source.as_deref() == Some("https://www.pixiv.net/en/artworks/12972758"));
    }
}
