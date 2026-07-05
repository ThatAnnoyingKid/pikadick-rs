INSERT INTO rule34_post (
    id,
    tags,
    file_url,
    last_fetched
) VALUES (
    :id,
    :tags,
    :file_url,
    :last_fetched
) ON CONFLICT (id) DO UPDATE SET
    file_url = :file_url,
    tags = :tags,
    last_fetched = :last_fetched;