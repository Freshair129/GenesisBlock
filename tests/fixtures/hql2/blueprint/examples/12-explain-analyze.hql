EXPLAIN ANALYZE USE kb MATCH (a:Service)-[:DEPENDS_ON*1..3]->(b) TRAIL
|> GROUP BY b.id AS id AGG count(*) AS paths
|> RETURN id, paths;
