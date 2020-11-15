use select::{
    document::Document,
    predicate::{
        And,
        Attr,
    },
};

#[derive(Debug)]
pub struct MainPage {
    pub csrf_key: String,
    pub csrf_value: String,
}

impl MainPage {
    pub(crate) fn from_doc(doc: &Document) -> Option<Self> {
        let download_form = doc.find(Attr("id", "download-form")).next()?;

        let (csrf_key, csrf_value) = download_form
            .find(And(Attr("name", ()), Attr("value", ())))
            .filter_map(|el| Some((el.attr("name")?, el.attr("value")?)))
            .find(|(name, _)| name != &"url")?;

        Some(MainPage {
            csrf_key: csrf_key.to_string(),
            csrf_value: csrf_value.to_string(),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const SAMPLE_1: &str = include_str!("../../test_data/main_page.html");

    #[test]
    fn parse() {
        let doc = Document::from(SAMPLE_1);
        let page = MainPage::from_doc(&doc).unwrap();
        dbg!(page);
    }
}
