SELECT
    query,
    queries_since_last_fetch,
    last_fetched
FROM
    rule34_query_stat
WHERE
    query = :query;