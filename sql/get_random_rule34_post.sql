SELECT
    rule34_post.id AS id,
    rule34_post.tags AS tags,
    rule34_post.file_url AS file_url,
    rule34_post.last_fetched AS last_fetched
FROM
    rule34_post
WHERE
    EXISTS (
        SELECT
            rule34_tag.name
        FROM
            rule34_tag
        JOIN
            rule34_post_tag
        ON
            rule34_post_tag.tag_id = rule34_tag.id
        WHERE
            rule34_post_tag.post_id = rule34_post.id AND
            (:tag_name IS NULL OR rule34_tag.name = :tag_name)
    )
ORDER BY
    SIN(rule34_post.id + :random_seed);