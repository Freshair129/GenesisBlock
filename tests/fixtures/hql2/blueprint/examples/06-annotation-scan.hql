USE kb FROM ANNOTATIONS AS a
|> FILTER a.kind = "review" AND a.confidence >= 0.8
|> ORDER BY a.created_at DESC
|> TAKE 50
|> RETURN a.id, a.targets, a.body;
