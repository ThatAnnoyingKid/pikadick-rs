const KEY: &str = include_str!("../key.txt");

#[tokio::test]
async fn random() {
    let client = fml::Client::new(KEY.into());
    let data = client.list_random(5).await.unwrap();
    println!("{:#?}", data);
    assert!(!data.is_empty());
}
