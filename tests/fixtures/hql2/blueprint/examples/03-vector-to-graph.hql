USE kb FROM NODES Document AS seed
|> FILTER seed.language = "th"
|> KNN seed IN docs USING $q TOP 50 APPROX AS hit
|> EXPAND (seed)-[:REFERENCES*1..2]->(x) AS path TRAIL
|> ANNOTATIONS OF x AS a
|> FILTER a.kind = "review" AND a.status = "approved"
|> GROUP BY x.id AS id AGG min(hit.distance) AS seed_distance, count(*) AS evidence_rows
|> ORDER BY seed_distance ASC, id ASC
|> TAKE 20
|> RETURN id, seed_distance, evidence_rows;
