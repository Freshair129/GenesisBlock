USE kb UNION ALL {
FROM NODES Document AS d |> FILTER d.language = "th" |> RETURN d.id AS id
} {
FROM NODES Document AS d |> FILTER d.language = "en" |> RETURN d.id AS id
}
|> DISTINCT
|> ORDER BY id ASC
|> RETURN id;
