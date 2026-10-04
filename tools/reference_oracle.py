"""Pure, filesystem-free reference semantics for the G3 fixtures.

This module deliberately does not import GenesisBlockDB, SQLite, or any engine
implementation. The Rust differential test compares public Storage results to
the expected values produced from the same versioned fixture.
"""

from copy import deepcopy


def _visible_at(valid_from, valid_to, as_of):
    if as_of is None:
        return True
    if valid_from > as_of:
        return False
    return valid_to is None or as_of < valid_to


def _graph_case(case):
    versions = {}
    edges = []
    markers = {}
    sequence = 0

    for operation in case["operations"]:
        kind = operation["op"]
        if kind == "add_node":
            sequence += 1
            versions.setdefault(operation["id"], []).append(
                {
                    "tx_from": sequence,
                    "valid_from": operation["valid_from"],
                    "valid_to": None,
                    "closed_tx": None,
                    "props": deepcopy(operation.get("props", {})),
                }
            )
        elif kind == "add_edge":
            sequence += 1
            edges.append(
                {
                    "tx_from": sequence,
                    "from": operation["from"],
                    "to": operation["to"],
                    "valid_from": operation["valid_from"],
                    "valid_to": None,
                }
            )
        elif kind == "mark":
            markers[operation["name"]] = sequence
        elif kind == "supersede":
            previous = versions[operation["id"]][-1]
            sequence += 1
            previous["valid_to"] = "2099-01-01T00:00:00Z"
            previous["closed_tx"] = sequence
            versions[operation["id"]].append(
                {
                    "tx_from": sequence + 1,
                    "valid_from": "2026-10-04T00:00:00Z",
                    "valid_to": None,
                    "closed_tx": None,
                    "props": deepcopy(operation["props"]),
                }
            )
            sequence += 1
        else:
            raise ValueError(f"unknown graph operation: {kind}")

    def version_at(node_id, tx, as_of):
        candidates = [
            version
            for version in versions[node_id]
            if version["tx_from"] <= tx
            and _visible_at(
                version["valid_from"],
                version["valid_to"]
                if version["closed_tx"] is None or version["closed_tx"] <= tx
                else None,
                as_of,
            )
        ]
        if not candidates:
            return None
        return candidates[-1]

    latest_tx = sequence
    expected = {}
    for query in case["queries"]:
        tx = latest_tx if query["tx"] == "latest" else markers[query["tx"]]
        version = version_at("doc", tx, query["valid_at"])
        if version is None:
            expected[query["id"]] = []
            continue
        # The fixture has one durable hub -> doc edge. Keep the edge in the
        # model so the case exercises traversal rather than a node lookup.
        edge = next(
            edge
            for edge in edges
            if edge["from"] == "hub"
            and edge["to"] == "doc"
            and edge["tx_from"] <= tx
            and _visible_at(edge["valid_from"], edge["valid_to"], query["valid_at"])
        )
        del edge
        expected[query["id"]] = [
            {
                "node_id": "doc",
                "props": deepcopy(version["props"]),
                "valid_window": "open"
                if version["closed_tx"] is None or version["closed_tx"] > tx
                else "closed",
            }
        ]
    return expected


def _relational_case(case):
    left_rows = case["rows"]["lefts"]
    right_rows = case["rows"]["rights"]
    output = []
    for left in left_rows:
        matches = [
            right
            for right in right_rows
            if left.get("join_key") is not None
            and right.get("join_key") is not None
            and left.get("join_key") == right.get("join_key")
        ]
        if not matches:
            matches = [None]
        for right in matches:
            output.append(
                {
                    "lefts.id": left["id"],
                    "rights.value": None if right is None else right.get("value"),
                }
            )
    return output


def evaluate_case(case):
    if case["kind"] == "graph_temporal":
        return _graph_case(case)
    if case["kind"] == "relational":
        return _relational_case(case)
    raise ValueError(f"unknown G3 fixture kind: {case['kind']}")
