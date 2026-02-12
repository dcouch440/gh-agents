"""Anthropic client, logging, and LLM call helpers."""

import json
import re
import time
from datetime import datetime
from pathlib import Path

import anthropic
from dotenv import load_dotenv

load_dotenv(Path(__file__).resolve().parents[2] / ".env")

MODEL = "claude-sonnet-4-5-20250929"
RESULTS_DIR = Path(__file__).resolve().parents[1]

client = anthropic.Anthropic()
call_log: list[dict] = []

_log_file: Path | None = None


def init_logging(log_path: Path):
    """Initialize logging to a file, clearing any previous content."""
    global _log_file
    _log_file = log_path
    _log_file.write_text("")


def log(msg: str, level: str = "INFO"):
    ts = datetime.now().strftime("%H:%M:%S.%f")[:-3]
    line = f"[{ts}] [{level:>5}] {msg}"
    print(line, flush=True)
    if _log_file:
        with open(_log_file, "a") as f:
            f.write(line + "\n")


def log_sep(title: str):
    log("")
    log("=" * 70)
    log(f"  {title}")
    log("=" * 70)


def call_text(system: str, user: str, label: str, max_tokens: int = 2048) -> dict:
    log(f"[LLM] {label} (text, max_tokens={max_tokens})")
    t0 = time.time()
    resp = client.messages.create(
        model=MODEL, max_tokens=max_tokens,
        system=system, messages=[{"role": "user", "content": user}],
    )
    ms = int((time.time() - t0) * 1000)
    text = resp.content[0].text
    stats = {
        "label": label, "type": "text",
        "input_tokens": resp.usage.input_tokens,
        "output_tokens": resp.usage.output_tokens,
        "ms": ms,
    }
    call_log.append(stats)
    log(f"  → {resp.usage.input_tokens} in / {resp.usage.output_tokens} out ({ms}ms)")
    return {"label": label, "text": text, **stats}


def _extract_json_fallback(text: str) -> dict | None:
    for m in re.finditer(r"\{", text):
        start = m.start()
        depth = 0
        for i in range(start, len(text)):
            if text[i] == "{":
                depth += 1
            elif text[i] == "}":
                depth -= 1
                if depth == 0:
                    try:
                        return json.loads(text[start:i + 1])
                    except json.JSONDecodeError:
                        break
    return None


def call_json(system: str, user: str, label: str, schema: dict,
              max_tokens: int = 4096) -> tuple[dict, dict]:
    log(f"[LLM] {label} (json, max_tokens={max_tokens})")
    t0 = time.time()
    resp = client.messages.create(
        model=MODEL, max_tokens=max_tokens,
        system=system, messages=[{"role": "user", "content": user}],
        tools=[{
            "name": "structured_output",
            "description": "Return structured data",
            "input_schema": schema,
        }],
        tool_choice={"type": "tool", "name": "structured_output"},
    )
    ms = int((time.time() - t0) * 1000)
    data = None
    for block in resp.content:
        if block.type == "tool_use":
            data = block.input
            break
    if data is None:
        text = resp.content[0].text if resp.content else ""
        data = _extract_json_fallback(text) or {}
        log(f"  [WARN] No tool_use block, used fallback extraction", "WARN")
    stats = {
        "label": label, "type": "json",
        "input_tokens": resp.usage.input_tokens,
        "output_tokens": resp.usage.output_tokens,
        "ms": ms,
    }
    call_log.append(stats)
    log(f"  → {resp.usage.input_tokens} in / {resp.usage.output_tokens} out ({ms}ms)")
    return data, stats


def save_incremental(data: dict, output_path: Path):
    with open(output_path, "w") as f:
        json.dump(data, f, indent=2, default=str)
