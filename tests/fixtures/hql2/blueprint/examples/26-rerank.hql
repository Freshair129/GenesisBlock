USE kb FROM NODES Document AS d
|> KNN d IN docs USING $q TOP 200 APPROX AS coarse
|> RERANK d IN docs USING $q TOP 20 EXACT AS fine
|> RETURN d.id, fine.distance;
