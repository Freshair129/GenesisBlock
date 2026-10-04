TRANSACTION ID $transaction_id {
UPSERT NODE "doc:one" LABELS ["Document"] PROPS $props;
UPSERT NODE "doc:two" LABELS ["Document"] PROPS $props2;
UPSERT EDGE "edge:one" FROM "doc:one" TO "doc:two" TYPE "REFERENCES" PROPS {};
UPSERT VECTOR ON NODE "doc:one" IN docs USING $q;
};
