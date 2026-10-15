USE kb MATCH (p:Project {id: "GenesisBlock"})-[:CONTAINS]->(x)
|> PROJECT x
|> DISTINCT
|> KNN x IN docs USING $q TOP 10 EXACT AS hit
|> RETURN x.id, hit.distance;
