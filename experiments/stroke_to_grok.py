"""
Send hand-drawn stroke coordinates to Grok and see if it can read them.

Usage:
    XAI_API_KEY=xai-... python3 experiments/stroke_to_grok.py
"""

import json
import os
import httpx

XAI_API_KEY = os.environ["XAI_API_KEY"]
XAI_URL = "https://api.x.ai/v1/chat/completions"


def send_to_grok(stroke_json: str, prompt: str) -> str:
    response = httpx.post(
        XAI_URL,
        headers={
            "Authorization": f"Bearer {XAI_API_KEY}",
            "Content-Type": "application/json",
        },
        json={
            "model": "grok-3-mini-fast",
            "messages": [
                {
                    "role": "system",
                    "content": (
                        "You are analyzing hand-drawn pen strokes represented as JSON coordinates. "
                        "The 'canvas' field is [width, height]. Each stroke has 'points' as [[x,y], ...] "
                        "where (0,0) is top-left. Identify what letter, word, number, or shape was drawn. "
                        "Be concise."
                    ),
                },
                {
                    "role": "user",
                    "content": f"{prompt}\n\n{stroke_json}",
                },
            ],
        },
        timeout=30.0,
    )
    response.raise_for_status()
    return response.json()["choices"][0]["message"]["content"]


# -- Letter definitions as stroke coordinates --------------------------------
# Each letter is drawn on a 100x100 canvas.

LETTERS = {
    "H": {
        "canvas": [100, 100],
        "strokes": [
            {"points": [[10, 10], [10, 90]]},       # left vertical
            {"points": [[10, 50], [90, 50]]},        # horizontal bar
            {"points": [[90, 10], [90, 90]]},        # right vertical
        ],
    },
    "A": {
        "canvas": [100, 100],
        "strokes": [
            {"points": [[10, 90], [50, 10], [90, 90]]},  # two legs
            {"points": [[30, 55], [70, 55]]},             # crossbar
        ],
    },
    "L": {
        "canvas": [100, 100],
        "strokes": [
            {"points": [[10, 10], [10, 90], [80, 90]]},  # vertical then horizontal
        ],
    },
    "O": {
        "canvas": [100, 100],
        "strokes": [
            {"points": [[50, 10], [85, 25], [90, 50], [85, 75], [50, 90],
                        [15, 75], [10, 50], [15, 25], [50, 10]]},
        ],
    },
    "T": {
        "canvas": [100, 100],
        "strokes": [
            {"points": [[10, 10], [90, 10]]},       # top bar
            {"points": [[50, 10], [50, 90]]},        # vertical stem
        ],
    },
    "X": {
        "canvas": [100, 100],
        "strokes": [
            {"points": [[10, 10], [90, 90]]},       # diagonal \
            {"points": [[90, 10], [10, 90]]},        # diagonal /
        ],
    },
    "1": {
        "canvas": [100, 100],
        "strokes": [
            {"points": [[30, 25], [50, 10], [50, 90]]},  # serif + stem
            {"points": [[30, 90], [70, 90]]},             # base
        ],
    },
    "star": {
        "canvas": [100, 100],
        "strokes": [
            {"points": [[50, 5], [60, 40], [95, 40], [68, 60], [78, 95],
                        [50, 72], [22, 95], [32, 60], [5, 40], [40, 40], [50, 5]]},
        ],
    },
    "house": {
        "canvas": [200, 200],
        "strokes": [
            {"points": [[30, 100], [30, 180], [170, 180], [170, 100]]},  # walls
            {"points": [[20, 100], [100, 30], [180, 100]]},              # roof
            {"points": [[80, 130], [80, 180], [120, 180], [120, 130], [80, 130]]},  # door
        ],
    },
}


def main():
    print("=" * 60)
    print("Stroke Coordinate Recognition via Grok")
    print("=" * 60)

    # Test individual letters
    for name, strokes in LETTERS.items():
        stroke_json = json.dumps(strokes)
        prompt = "What letter, number, or shape is this?"
        print(f"\n--- Sending: '{name}' ({len(stroke_json)} chars) ---")

        try:
            answer = send_to_grok(stroke_json, prompt)
            print(f"Grok says: {answer}")
        except Exception as e:
            print(f"Error: {e}")

    # Test a word: "HI" (two letters side by side on a wider canvas)
    word = {
        "canvas": [200, 100],
        "strokes": [
            # H
            {"points": [[10, 10], [10, 90]]},
            {"points": [[10, 50], [60, 50]]},
            {"points": [[60, 10], [60, 90]]},
            # I
            {"points": [[100, 10], [140, 10]]},
            {"points": [[120, 10], [120, 90]]},
            {"points": [[100, 90], [140, 90]]},
        ],
    }
    print(f"\n--- Sending word: 'HI' ({len(json.dumps(word))} chars) ---")
    try:
        answer = send_to_grok(json.dumps(word), "What word do these strokes spell?")
        print(f"Grok says: {answer}")
    except Exception as e:
        print(f"Error: {e}")

    print("\n" + "=" * 60)


if __name__ == "__main__":
    main()
