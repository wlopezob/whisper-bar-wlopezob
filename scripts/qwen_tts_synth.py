#!/usr/bin/env python3
"""Síntesis Qwen3-TTS (VoiceDesign, MLX) para whisper-bar.

Embebido en el binario Rust (include_str!) y ejecutado con el venv de
~/.config/whisperwlopezob/tts-venv. Recibe JSON por stdin:

    {"text": "...", "instruct": "...", "language": "spanish",
     "temperature": 0.7, "model_dir": "/abs/path", "out_path": "/abs/out.wav"}

Escribe el WAV en out_path e imprime "OK <segundos>" por stdout.
Cualquier error sale por stderr con exit code 1.
"""

import json
import sys


def main():
    cfg = json.load(sys.stdin)

    from mlx_audio.tts.utils import load_model
    from mlx_audio.audio_io import write as audio_write

    model = load_model(cfg["model_dir"])

    results = list(model.generate_voice_design(
        text=cfg["text"],
        language=cfg.get("language", "spanish"),
        instruct=cfg["instruct"],
        temperature=float(cfg.get("temperature", 0.7)),
    ))

    audio = results[0].audio
    sr = results[0].sample_rate
    audio_write(cfg["out_path"], audio, sr)
    print(f"OK {audio.shape[0] / sr:.1f}")


if __name__ == "__main__":
    try:
        main()
    except Exception as e:
        print(f"qwen-tts-synth error: {e}", file=sys.stderr)
        sys.exit(1)
