USE kb FROM NODES Document AS d
|> FILTER (d.language IN ["th", "en"] OR d.language IS NULL) AND NOT d.archived = true
|> RETURN d.id;
