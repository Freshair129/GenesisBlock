#!/usr/bin/env python3
"""Minimal, deterministic CPU embedding sidecar for the pinned E5 model.

The Node worker verifies the revision and every artifact hash before starting
this process. This sidecar only loads those exact paths and has no fallback
model or network behavior.
"""

import json
import os
import sys
import traceback

import numpy as np
import onnxruntime as ort
from tokenizers import Tokenizer


MODEL_DIMENSIONS = 384
MAX_LENGTH = 512


def fail(message):
    raise RuntimeError(message)


def load(model_dir):
    tokenizer_path = os.path.join(model_dir, "tokenizer.json")
    model_path = os.path.join(model_dir, "onnx", "model.onnx")
    tokenizer = Tokenizer.from_file(tokenizer_path)
    tokenizer.enable_truncation(max_length=MAX_LENGTH)
    session = ort.InferenceSession(model_path, providers=["CPUExecutionProvider"])
    input_names = {item.name for item in session.get_inputs()}
    if input_names != {"input_ids", "attention_mask", "token_type_ids"}:
        fail("ONNX_INPUT_SCHEMA_MISMATCH")
    outputs = session.get_outputs()
    if len(outputs) != 1 or outputs[0].name != "last_hidden_state":
        fail("ONNX_OUTPUT_SCHEMA_MISMATCH")
    shape = outputs[0].shape
    if len(shape) != 3 or shape[-1] != MODEL_DIMENSIONS:
        fail("ONNX_OUTPUT_DIMENSION_MISMATCH")
    return tokenizer, session


def embed(tokenizer, session, texts, mode):
    if mode not in ("query", "passage"):
        fail("EMBED_MODE_INVALID")
    prefix = "query: " if mode == "query" else "passage: "
    encoded = tokenizer.encode_batch([prefix + text for text in texts])
    if not encoded:
        return []
    max_len = max(len(item.ids) for item in encoded)
    max_len = min(max_len, MAX_LENGTH)
    ids = np.zeros((len(encoded), max_len), dtype=np.int64)
    attention = np.zeros((len(encoded), max_len), dtype=np.int64)
    type_ids = np.zeros((len(encoded), max_len), dtype=np.int64)
    for row, item in enumerate(encoded):
        size = min(len(item.ids), max_len)
        ids[row, :size] = item.ids[:size]
        attention[row, :size] = item.attention_mask[:size]
        type_ids[row, :size] = item.type_ids[:size]
    output = session.run(["last_hidden_state"], {
        "input_ids": ids,
        "attention_mask": attention,
        "token_type_ids": type_ids,
    })[0]
    mask = attention.astype(np.float32)[..., None]
    pooled = (output * mask).sum(axis=1) / np.maximum(mask.sum(axis=1), 1e-9)
    norms = np.linalg.norm(pooled, axis=1, keepdims=True)
    if np.any(norms <= 0) or not np.isfinite(norms).all():
        fail("EMBEDDING_ZERO_OR_NONFINITE")
    normalized = pooled / norms
    return normalized.astype(np.float64).tolist()


def main():
    if len(sys.argv) != 2:
        print(json.dumps({"error": "MODEL_DIR_REQUIRED"}), flush=True)
        return 2
    try:
        tokenizer, session = load(sys.argv[1])
        print(json.dumps({"ready": True, "dimensions": MODEL_DIMENSIONS, "provider": "CPUExecutionProvider"}), flush=True)
        for line in sys.stdin:
            if not line.strip():
                continue
            request_id = None
            try:
                request = json.loads(line)
                request_id = request.get("id")
                texts = request.get("texts")
                if not isinstance(texts, list) or any(not isinstance(text, str) for text in texts):
                    fail("EMBED_TEXTS_INVALID")
                vectors = embed(tokenizer, session, texts, request.get("mode", "passage"))
                print(json.dumps({"id": request_id, "vectors": vectors, "dimensions": MODEL_DIMENSIONS}, separators=(",", ":")), flush=True)
            except Exception as error:
                print(json.dumps({"id": request_id, "error": str(error)}, separators=(",", ":")), flush=True)
    except Exception as error:
        print(json.dumps({"error": str(error), "trace": traceback.format_exc(limit=1)}), flush=True)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
