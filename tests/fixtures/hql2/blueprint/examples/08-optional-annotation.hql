USE kb FROM NODES Document AS d
|> OPTIONAL ANNOTATIONS OF d AS a
|> RETURN d.id AS document_id, a.id AS annotation_id;
