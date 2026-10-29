INSERT INTO rule34_tag (
    name
) VALUES (
    :name
) ON CONFLICT (name) DO UPDATE SET
    name = :name
RETURNING
    id;