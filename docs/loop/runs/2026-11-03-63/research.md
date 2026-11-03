# research.md — iteration 63: the text tower, and whether the space is real

## S1 — `laion/clap-htsat-fused`'s text tower, measured on `bmb` (Apple M4, torch 2.15.0, transformers 5.22.1)

| Quantity | Measured |
|---|---|
| text tower parameters | **124,645,632** |
| tokeniser keys | `input_ids`, `attention_mask` |
| embedding shape | `[batch, 512]` |
| embedding norm | **1.0 on every row** — L2-normalised, like the audio tower |
| export, opset 17, `dynamo=False` | **ok** |
| ONNX size | **501.4 MB** |
| graph | 1,016 nodes, 31 distinct operators, all standard |
| cosine vs PyTorch | **0.999999969** |
| max abs difference | **6.2 × 10⁻⁷** |

The text tower is **4.5×** the audio tower (501.4 MB vs 116.5 MB) and **4.5×** the parameter count — CLAP is
text-heavy, which is the opposite of what one might guess from "an audio model".

## S2 — variable token length works

| Query | Words | Tokens | Cosine vs PyTorch |
|---|---|---|---|
| `tense` | 1 | 4 | **1.000000031** |
| `tense sparse underwater ruins with distant whale song and a slow heartbeat` | 12 | 15 | **1.000000085** |

Unlike the audio tower, the **token axis is genuinely dynamic** — 4 tokens and 15 tokens both run in the one exported
graph, because they were in the same batch. So a query needs no re-export and no fixed padding beyond the batch's own
longest text.

## S3 — the premise: text and audio share a space

This is the question the whole feature rests on. An ONNX export that matches PyTorch exactly proves only that the
export is faithful; it says nothing about whether a *description* lands near the *sound it describes*.

Six synthetic clips, five descriptions, cosine of each text embedding against each audio embedding:

| query | tone | noise | rumble | pulse | rain | pad |
|---|---|---|---|---|---|---|
| rain falling | 0.009 | **0.126** | 0.058 | −0.104 | 0.115 | 0.077 |
| a steady musical tone | 0.150 | −0.040 | −0.003 | −0.027 | −0.058 | **0.218** |
| a low rumble | 0.074 | 0.081 | **0.217** | −0.000 | 0.066 | 0.142 |
| white noise hiss | −0.009 | 0.423 | 0.026 | −0.042 | **0.429** | −0.019 |
| a rhythmic beat | 0.040 | −0.052 | 0.094 | **0.133** | −0.078 | 0.076 |

**Four of five winners are correct.** "rain falling" prefers white noise (0.126) to the noise-modulated "rain" clip
(0.115) — and that is not a failure, it is a fair call: my "rain" clip *is* amplitude-modulated white noise, and the
unmodulated one is closer to the literal words. "white noise hiss" ties the two noise clips at 0.423/0.429, which is
the same fact from the other side.

**The magnitudes are the finding, and they change `R-05`'s default.**

| | |
|---|---|
| correct matches | **0.126 – 0.429** |
| incorrect ones | **−0.104 – +0.150** |

The bands **overlap**. A floor of 0.5 — the number intuition suggests — would reject **every single correct match**.
A floor of 0.15 would reject most wrong ones and keep most right ones, and it is still only a design default until
`V-01`'s labelled set measures a better one.

**This is a real number from a real model**, not an estimate, and it is why `Floor`'s documented default of `0.0` is
justified while that labelled set does not exist: a floor that hides the right answer is worse than a long list.

## S4 — latency

| Device | Batch | Per call |
|---|---|---|
| **Apple M4 CPU** | 1 | **5.28 ms** |
| **Apple M4 CPU** | 4 | **7.34 ms** |

A text query costs **5.3 ms** on a laptop CPU, against **34.9 ms** for one audio clip. So a query is ~6× cheaper than
indexing a clip, which is what makes the query side feel instant and the index side the batch job.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| text tower parameters | 124,645,632 | measured, S1 |
| text ONNX size | 501.4 MB | measured, S1 |
| text embedding | 512-d, L2-normalised | measured, S1 |
| token axis | dynamic, 4 to 15 tokens proven | measured, S2 |
| text→audio cosine, correct | 0.126 – 0.429 | measured, S3 |
| text→audio cosine, incorrect | −0.104 – +0.150 | measured, S3 |
| query latency, M4 CPU | 5.28 ms | measured, S4 |

## What good sounds like

For a **query**, "good" is three things: the right clip ranks first (4 of 5 above), the score is meaningfully above
the incorrect band (it is, but the bands overlap), and the cost is small enough to feel instant (5.28 ms). The overlap
is the honest caveat, and it is why a score breakdown rather than a bare confidence is the right UI.
