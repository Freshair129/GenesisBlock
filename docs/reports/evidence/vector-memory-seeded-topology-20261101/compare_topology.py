import hashlib, json, sys
from pathlib import Path

def read(path):
    raw = Path(path).read_bytes()
    points = {}
    for line in raw.decode('ascii').splitlines():
        fields = line.split(' |')
        node, level, slot = map(int, fields[0].split())
        if node in points:
            raise ValueError('duplicate node')
        layers = []
        for layer, group in enumerate(fields[1:]):
            words = group.split()
            if int(words[0]) != layer:
                raise ValueError('missing or reordered layer')
            layers.append(tuple(words[1:]))
        points[node] = (level, slot, tuple(layers))
    return hashlib.sha256(raw).hexdigest(), points

def compare(a, b):
    ah, ap = read(a); bh, bp = read(b)
    shared = ap.keys() & bp.keys()
    level_diff = sum(ap[i][0] != bp[i][0] for i in shared)
    slot_diff = sum(ap[i][1] != bp[i][1] for i in shared)
    ordered = sum(ap[i][2] != bp[i][2] for i in shared)
    def neighbor_ids(layers):
        return tuple(frozenset(int(item.split(':', 1)[0]) for item in layer) for layer in layers)
    sets = sum(neighbor_ids(ap[i][2]) != neighbor_ids(bp[i][2]) for i in shared)
    return {'before_sha256':ah, 'after_sha256':bh, 'identical_bytes':ah == bh,
            'before_nodes':len(ap), 'after_nodes':len(bp), 'shared_nodes':len(shared),
            'missing_before':len(bp.keys()-ap.keys()), 'missing_after':len(ap.keys()-bp.keys()),
            'level_different_nodes':level_diff, 'slot_different_nodes':slot_diff,
            'ordered_neighbor_records_different_nodes':ordered, 'neighbor_id_set_different_nodes':sets}

if __name__ == '__main__':
    print(json.dumps(compare(*sys.argv[1:]), indent=2))
