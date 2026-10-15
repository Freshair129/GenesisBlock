USE kb FROM NODES Document AS d
|> FILTER d.language = "th" AND d.published_at >= timestamp("2026-01-01T00:00:00Z")
|> KNN d IN docs USING $q TOP 20 EXACT AS hit
|> RETURN d.id, hit.distance;
