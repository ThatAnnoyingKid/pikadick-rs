use time::format_description::FormatItem;

pub(crate) const ASCTIME_WITH_OFFSET_FORMAT: &[FormatItem<'_>] = time::macros::format_description!(
    "[weekday repr:short] [month repr:short] [day] [hour]:[minute]:[second] [offset_hour][offset_minute] [year]"
);

pub(crate) mod asctime_with_offset {
    time::serde::format_description!(
        asctime_with_offset_impl,
        OffsetDateTime,
        ASCTIME_WITH_OFFSET_FORMAT
    );

    pub use self::asctime_with_offset_impl::*;
    use super::ASCTIME_WITH_OFFSET_FORMAT;
}

#[cfg(test)]
mod test {
    use super::*;
    use time::OffsetDateTime;

    #[test]
    fn asctime_with_offset_sanity() {
        let date_time_str = "Sat Sep 02 02:01:00 +0000 2023";
        let date = OffsetDateTime::parse(date_time_str, ASCTIME_WITH_OFFSET_FORMAT)
            .expect("failed to parse");

        assert!(date.unix_timestamp() == 1693620060);
    }
}
