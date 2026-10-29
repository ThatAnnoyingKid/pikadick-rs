pub mod platform;
/// User Data Type
pub mod user_data;

pub use self::{
    platform::Platform,
    user_data::{
        ApiResponse,
        InvalidApiResponseError,
        SegmentSeason,
        UserData,
    },
};
