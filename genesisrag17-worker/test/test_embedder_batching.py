"""Dependency-free contract tests for the E5 sidecar's ORT batch boundary."""

import importlib.util
import sys
import types
import unittest
from pathlib import Path
from unittest.mock import patch


class FakeArray:
    def __init__(self, shape, row_values=None):
        self.shape = tuple(shape)
        rows = self.shape[0] if self.shape else 0
        self.row_values = list(row_values) if row_values is not None else [0] * rows
        self.values = (
            [[0] * self.shape[1] for _ in range(rows)]
            if len(self.shape) == 2 else None
        )

    def __setitem__(self, key, value):
        row, columns = key
        self.values[row][columns] = list(value)

    def __getitem__(self, key):
        if key == (Ellipsis, None):
            return FakeArray((*self.shape, 1), self.row_values)
        raise AssertionError(f"unexpected tensor index: {key!r}")

    def __mul__(self, other):
        return FakeArray(self.shape, self.row_values)

    def __truediv__(self, other):
        return FakeArray(self.shape, self.row_values)

    def __le__(self, other):
        return FakeTruth()

    def astype(self, dtype):
        return self

    def sum(self, axis):
        shape = list(self.shape)
        del shape[axis]
        return FakeArray(shape, self.row_values)

    def tolist(self):
        if len(self.shape) != 2:
            raise AssertionError(f"unexpected output shape: {self.shape!r}")
        return [[float(value)] * self.shape[1] for value in self.row_values]


class FakeTruth:
    def all(self):
        return True


class FakeNumpy(types.ModuleType):
    int64 = object()
    float32 = object()
    float64 = object()

    def __init__(self):
        super().__init__("numpy")
        self.linalg = types.SimpleNamespace(
            norm=lambda array, axis, keepdims: FakeArray(
                (array.shape[0], 1), [1] * array.shape[0]
            )
        )

    def zeros(self, shape, dtype):
        return FakeArray(shape)

    def maximum(self, array, minimum):
        return array

    def any(self, value):
        return False

    def isfinite(self, value):
        return FakeTruth()


class FakeEncoding:
    def __init__(self, marker, length):
        self.ids = [marker] + [marker + 1000] * (length - 1)
        self.attention_mask = [1] * length
        self.type_ids = [0] * length


class FakeTokenizer:
    def __init__(self, lengths=None):
        self.inputs = []
        self.lengths = lengths

    def encode_batch(self, texts):
        self.inputs = list(texts)
        lengths = self.lengths or [2] * len(texts)
        return [
            FakeEncoding(index + 1, lengths[index])
            for index, _ in enumerate(texts)
        ]


class FakeSession:
    def __init__(self, fail_on_call=None):
        self.batch_sizes = []
        self.feed_shapes = []
        self.output_names = []
        self.fail_on_call = fail_on_call

    def run(self, output_names, feeds):
        self.output_names.append(list(output_names))
        batch_size = feeds["input_ids"].shape[0]
        self.batch_sizes.append(batch_size)
        self.feed_shapes.append({name: value.shape for name, value in feeds.items()})
        if self.fail_on_call == len(self.batch_sizes):
            raise RuntimeError("FAKE_ORT_FAILURE")
        markers = [row[0] for row in feeds["input_ids"].values]
        output = FakeArray(
            (batch_size, feeds["input_ids"].shape[1], 384), markers
        )
        return [output]

def load_sidecar_with_fakes():
    fake_ort = types.ModuleType("onnxruntime")
    fake_ort.InferenceSession = lambda *args, **kwargs: None
    fake_tokenizers = types.ModuleType("tokenizers")
    fake_tokenizers.Tokenizer = FakeTokenizer
    fake_numpy = FakeNumpy()
    path = Path(__file__).parents[1] / "src" / "embedder.py"
    spec = importlib.util.spec_from_file_location("test_embedder_sidecar", path)
    module = importlib.util.module_from_spec(spec)
    with patch.dict(sys.modules, {
        "numpy": fake_numpy,
        "onnxruntime": fake_ort,
        "tokenizers": fake_tokenizers,
    }):
        spec.loader.exec_module(module)
    return module, fake_numpy


def run_embedder(tokenizer, session, texts, mode):
    with patch.dict(sys.modules, {"numpy": fake_numpy}):
        return embedder.embed(tokenizer, session, texts, mode)


embedder, fake_numpy = load_sidecar_with_fakes()

class EmbedderBatchingTests(unittest.TestCase):
    def test_passage_requests_use_bounded_ordered_batches(self):
        for count in (0, 1, 32, 33, 65, 302):
            with self.subTest(count=count):
                tokenizer = FakeTokenizer()
                session = FakeSession()
                texts = [f"item-{index}" for index in range(count)]

                vectors = run_embedder(tokenizer, session, texts, "passage")

                expected_batches = [
                    min(32, count - start) for start in range(0, count, 32)
                ]
                self.assertEqual(session.batch_sizes, expected_batches)
                self.assertTrue(all(size <= 32 for size in session.batch_sizes))
                self.assertEqual(
                    tokenizer.inputs,
                    [f"passage: {text}" for text in texts],
                )
                self.assertEqual([vector[0] for vector in vectors], [
                    float(index + 1) for index in range(count)
                ])
                self.assertTrue(all(len(vector) == 384 for vector in vectors))
                self.assertTrue(all(
                    names == ["last_hidden_state"]
                    for names in session.output_names
                ))
                self.assertTrue(all(
                    set(shapes) == {"input_ids", "attention_mask", "token_type_ids"}
                    and len(set(shapes.values())) == 1
                    for shapes in session.feed_shapes
                ))

    def test_each_batch_pads_to_its_own_longest_sequence(self):
        lengths = [2] * 32 + [5]
        tokenizer = FakeTokenizer(lengths)
        session = FakeSession()
        texts = [f"item-{index}" for index in range(33)]

        run_embedder(tokenizer, session, texts, "passage")

        self.assertEqual(session.batch_sizes, [32, 1])
        self.assertEqual(
            [shapes["input_ids"] for shapes in session.feed_shapes],
            [(32, 2), (1, 5)],
        )

    def test_later_ort_failure_propagates_without_partial_result(self):
        tokenizer = FakeTokenizer()
        session = FakeSession(fail_on_call=2)
        texts = [f"item-{index}" for index in range(33)]

        with self.assertRaisesRegex(RuntimeError, "FAKE_ORT_FAILURE"):
            run_embedder(tokenizer, session, texts, "passage")

        self.assertEqual(session.batch_sizes, [32, 1])
    def test_query_prefix_and_singleton_request_are_unchanged(self):
        tokenizer = FakeTokenizer()
        session = FakeSession()

        vectors = run_embedder(tokenizer, session, ["query text"], "query")

        self.assertEqual(tokenizer.inputs, ["query: query text"])
        self.assertEqual(session.batch_sizes, [1])
        self.assertEqual(len(vectors), 1)
        self.assertEqual(len(vectors[0]), 384)


if __name__ == "__main__":
    unittest.main()