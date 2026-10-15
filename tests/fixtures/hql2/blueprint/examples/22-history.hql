USE kb HISTORY NODE "doc:one" AS h
|> ORDER BY h.tx_from ASC
|> RETURN h.revision_id, h.tx_from, h.tx_to, h.valid_from, h.valid_to;
