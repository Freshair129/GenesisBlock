USE kb FROM NODES Document AS d
|> OPTIONAL ANNOTATIONS OF d AS a
|> RETURN d.id, a.id;
