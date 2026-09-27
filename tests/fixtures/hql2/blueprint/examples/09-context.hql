USE kb FROM NODES Document AS d
|> KNN d IN docs USING $q TOP 30 APPROX AS hit
|> PACK CONTEXT TEXT d.text EVIDENCE d TOKENS 4000 TOKENIZER "registered-tokenizer-v1" AS package
|> RETURN package;
