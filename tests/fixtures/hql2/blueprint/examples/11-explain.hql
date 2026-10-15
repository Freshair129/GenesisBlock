EXPLAIN USE kb FROM NODES Document AS d
|> FILTER d.language = "th"
|> KNN d IN docs USING $q TOP 10 EXACT AS h
|> RETURN d.id, h.distance;
