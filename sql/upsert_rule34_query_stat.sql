INSERT INTO rule34_query_stat (
    query,
    queries_since_last_fetch,
    last_fetched
) VALUES (
    :query,
    :queries_since_last_fetch,
    :last_fetched
) ON CONFLICT (query) DO UPDATE SET
    query = :query,
    last_fetched = :last_fetched;