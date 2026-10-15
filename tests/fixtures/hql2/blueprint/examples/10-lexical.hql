USE kb FROM NODES Document AS d
|> FILTER d.language = "th"
|> LEXICAL d FIELD text QUERY $text USING INDEX docs_text TOP 20 AS lexical_hit
|> RETURN d.id, lexical_hit.score, lexical_hit.rank;
