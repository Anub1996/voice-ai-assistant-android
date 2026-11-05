"""Independent numpy reference forward pass for GGUF-convention models.

Reads raw f32 weights dumped by the Rust test (one .bin per tensor plus
spec.json) and prints next-token logits for the given token ids as JSON.
Implements llama.cpp semantics: RMSNorm with plain weight multiply,
RoPE "NORM" (interleaved pairs) for llama, "NEOX" (split halves) for
qwen2/qwen3, optional QKV bias (qwen2) and per-head Q/K RMSNorm (qwen3),
grouped-query attention, SwiGLU MLP, tied or separate output head.
"""
import json, os, sys
import numpy as np

d = sys.argv[1]
tokens = [int(t) for t in sys.argv[2].split(",")]
spec = json.load(open(os.path.join(d, "spec.json")))

def w(name):
    shape = spec["tensors"][name]
    return np.fromfile(os.path.join(d, name + ".bin"), dtype="<f4").reshape(shape).astype(np.float64)

def has(name):
    return name in spec["tensors"]

def rms(x, g, eps):
    return x / np.sqrt((x * x).mean(-1, keepdims=True) + eps) * g

H, KV, hd, L = spec["heads"], spec["kv_heads"], spec["head_dim"], spec["layers"]
eps, theta, arch = spec["eps"], spec["theta"], spec["arch"]
T = len(tokens)
pos = np.arange(T)
inv = theta ** (-np.arange(0, hd, 2) / hd)          # (hd/2,)
ang = pos[:, None] * inv[None, :]                    # (T, hd/2)
cos, sin = np.cos(ang), np.sin(ang)

def rope(x):  # x: (T, n, hd)
    c, s = cos[:, None, :], sin[:, None, :]
    if arch == "llama":
        a, b = x[..., 0::2], x[..., 1::2]
        out = np.empty_like(x)
        out[..., 0::2] = a * c - b * s
        out[..., 1::2] = a * s + b * c
        return out
    a, b = x[..., : hd // 2], x[..., hd // 2 :]
    return np.concatenate([a * c - b * s, a * s + b * c], -1)

x = w("token_embd.weight")[tokens]
for l in range(L):
    p = f"blk.{l}."
    h = rms(x, w(p + "attn_norm.weight"), eps)
    q, k, v = h @ w(p + "attn_q.weight").T, h @ w(p + "attn_k.weight").T, h @ w(p + "attn_v.weight").T
    if has(p + "attn_q.bias"):
        q, k, v = q + w(p + "attn_q.bias"), k + w(p + "attn_k.bias"), v + w(p + "attn_v.bias")
    q, k, v = q.reshape(T, H, hd), k.reshape(T, KV, hd), v.reshape(T, KV, hd)
    if has(p + "attn_q_norm.weight"):
        q, k = rms(q, w(p + "attn_q_norm.weight"), eps), rms(k, w(p + "attn_k_norm.weight"), eps)
    q, k = rope(q), rope(k)
    rep = H // KV
    k, v = np.repeat(k, rep, axis=1), np.repeat(v, rep, axis=1)
    scores = np.einsum("thd,shd->hts", q, k) / np.sqrt(hd)
    scores = scores + np.triu(np.full((T, T), -np.inf), 1)[None]
    scores = np.exp(scores - scores.max(-1, keepdims=True))
    att = scores / scores.sum(-1, keepdims=True)
    o = np.einsum("hts,shd->thd", att, v).reshape(T, H * hd)
    x = x + o @ w(p + "attn_output.weight").T
    h = rms(x, w(p + "ffn_norm.weight"), eps)
    g, u = h @ w(p + "ffn_gate.weight").T, h @ w(p + "ffn_up.weight").T
    x = x + ((g / (1 + np.exp(-g))) * u) @ w(p + "ffn_down.weight").T
x = rms(x, w("output_norm.weight"), eps)
out = w("output.weight") if has("output.weight") else w("token_embd.weight")
print(json.dumps((x[-1] @ out.T).tolist()))
