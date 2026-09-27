"""Small independent semantic oracle; not a GenesisBlockDB implementation."""
from __future__ import annotations
from datetime import datetime, timezone
import math
import struct
from typing import Any, Iterable


def instant_us(text: str) -> int:
    dt = datetime.fromisoformat(text.replace('Z', '+00:00'))
    if dt.tzinfo is None:
        raise ValueError('TIMEZONE_REQUIRED')
    delta = dt.astimezone(timezone.utc) - datetime(1970, 1, 1, tzinfo=timezone.utc)
    return (delta.days * 86400 + delta.seconds) * 1_000_000 + delta.microseconds


def visible(revision: dict[str, Any], tx: int, valid: int) -> bool:
    return (revision['tx_from'] <= tx
            and (revision.get('tx_to') is None or tx < revision['tx_to'])
            and revision['valid_from'] <= valid
            and (revision.get('valid_to') is None or valid < revision['valid_to'])
            and not revision.get('retracted', False))


def distance(a: Iterable[float], b: Iterable[float], metric: str = 'l2_squared') -> float:
    av, bv = list(a), list(b)
    if not av or len(av) != len(bv):
        raise ValueError('DIMENSION_MISMATCH')
    if not all(math.isfinite(x) for x in av + bv):
        raise ValueError('NONFINITE_VECTOR')
    if metric == 'l2_squared':
        return math.fsum((x-y)**2 for x, y in zip(av, bv))
    dot = math.fsum(x*y for x,y in zip(av,bv))
    if metric == 'neg_dot':
        return -dot
    if metric == 'cosine':
        na = math.sqrt(math.fsum(x*x for x in av))
        nb = math.sqrt(math.fsum(x*x for x in bv))
        if na == 0 or nb == 0:
            raise ValueError('ZERO_COSINE_NORM')
        return 1.0 - max(-1.0, min(1.0, dot / (na * nb)))
    raise ValueError('METRIC_UNSUPPORTED')


def exact_topk(rows: list[dict[str, Any]], q: list[float], k: int,
               metric: str = 'l2_squared') -> list[dict[str, Any]]:
    if k < 0:
        raise ValueError('INVALID_K')
    # Query validation does not depend on whether the input bag is empty.
    distance(q, q, metric)
    scored = [(distance(r['vector'], q, metric), r['row_key'], r)
              for r in rows if r.get('vector') is not None]
    return [r for _, _, r in sorted(scored, key=lambda x: (x[0], x[1]))[:k]]


def sql_and(a: bool | None, b: bool | None) -> bool | None:
    if a is False or b is False:
        return False
    if a is None or b is None:
        return None
    return True


def sql_or(a: bool | None, b: bool | None) -> bool | None:
    if a is True or b is True:
        return True
    if a is None or b is None:
        return None
    return False


def key_i64(value: int) -> bytes:
    if not -(1 << 63) <= value < (1 << 63):
        raise ValueError('I64_OVERFLOW')
    return ((value & ((1 << 64)-1)) ^ (1 << 63)).to_bytes(8, 'big')


def key_f64(value: float) -> bytes:
    if not math.isfinite(value):
        raise ValueError('NONFINITE_KEY')
    value = 0.0 if value == 0 else value
    bits = int.from_bytes(struct.pack('>d', value), 'big')
    ordered = (~bits & ((1 << 64)-1)) if bits >> 63 else bits ^ (1 << 63)
    return ordered.to_bytes(8, 'big')


def key_utf8(value: str) -> bytes:
    return value.encode('utf-8').replace(b'\x00', b'\x00\xff') + b'\x00\x00'


def validate_dag(plan: dict[str, Any]) -> None:
    nodes = plan['nodes']
    if not 1 <= len(nodes) <= 10000:
        raise ValueError('IR_SIZE')
    mapping = {n['id']: n for n in nodes}
    if len(mapping) != len(nodes):
        raise ValueError('DUPLICATE_NODE_ID')
    root = plan['root']
    if root not in mapping:
        raise ValueError('ROOT_MISSING')
    for n in nodes:
        if any(inp not in mapping for inp in n['inputs']):
            raise ValueError('INPUT_MISSING')
    state: dict[str, int] = {}
    stack = [(root, False, 0)]
    while stack:
        ident, finish, depth = stack.pop()
        if depth > 128:
            raise ValueError('IR_DEPTH')
        if finish:
            state[ident] = 2
            continue
        if state.get(ident) == 1:
            raise ValueError('IR_CYCLE')
        if state.get(ident) == 2:
            continue
        state[ident] = 1
        stack.append((ident, True, depth))
        for inp in reversed(mapping[ident]['inputs']):
            stack.append((inp, False, depth + 1))
    if len(state) != len(nodes):
        raise ValueError('UNREACHABLE_NODE')


def check_annotation_target(annotation: dict[str, Any], revisions: dict[str, dict[str, Any]]) -> None:
    for target in annotation['targets']:
        ref = target['ref']
        if ref['namespace'] != annotation['namespace']:
            raise ValueError('CROSS_NAMESPACE_TARGET')
        if target['binding'] == 'frozen':
            rev = revisions.get(ref.get('revision_id', ''))
            if rev is None or rev['id'] != ref['id'] or rev['namespace'] != ref['namespace']:
                raise ValueError('TARGET_REVISION_MISMATCH')
        selector = target['selector']
        if selector['type'] in ('text_position', 'byte_range'):
            if selector['start'] >= selector['end']:
                raise ValueError('EMPTY_OR_REVERSED_SELECTOR')


def authorized_annotation(annotation_id: str, target_id: str, allowed: set[str]) -> bool:
    return annotation_id in allowed and target_id in allowed
