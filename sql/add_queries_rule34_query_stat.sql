UPDATE
    rule34_query_stat
SET
    queries_since_last_fetch = queries_since_last_fetch + :value
WHERE
    query = :query;