#[tokio::test]
async fn check_room() {
    let client = quizizz::Client::new();
    let data = client.check_room("114545").await.unwrap();

    dbg!(data);
}
