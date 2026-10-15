USE kb FROM NODES Document AS d
|> KNN d IN docs USING $q TOP 20 EXACT AS hit
|> FILTER d.language = "th"
|> RETURN d.id, hit.distance;
