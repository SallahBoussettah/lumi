"""
Omniscient TTS sidecar — Kokoro-82M served over HTTP.

Uses the official `kokoro` PyTorch package, which exposes real per-word
timestamps via the model's `pred_dur` output (aligned to misaki word tokens).
This is the same alignment data Kokoro-FastAPI's /dev/captioned_speech uses,
and accounts for actual model pacing including comma/period pauses — much
more accurate than char-ratio approximation.

Endpoints:
- GET  /health           -> {ok, voice, sample_rate, model_loaded}
- GET  /voices           -> list of voice ids
- POST /tts              -> {text, voice?, speed?} -> JSON {audio_b64, sample_rate, words}

Run:
  uv sync
  uv run python server.py [--port 17891]
"""

import argparse
import base64
import io
import logging
import re
import sys
from contextlib import asynccontextmanager
from pathlib import Path

# A token is "wordy" if it contains at least one letter or digit. Bare-
# punctuation tokens ("." "," "?") get their own entries from KPipeline; we
# drop them so the client's per-word index aligns with how display text is
# split on whitespace (where punctuation is glued to the preceding word).
_WORDY_RE = re.compile(r"\w", re.UNICODE)

import numpy as np
import soundfile as sf
import uvicorn
from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel

logging.basicConfig(
    level=logging.INFO,
    format="[tts] %(asctime)s %(levelname)s %(message)s",
    datefmt="%H:%M:%S",
)
log = logging.getLogger("tts")

DEFAULT_VOICE = "af_heart"
DEFAULT_LANG = "a"  # 'a' = American English, 'b' = British, etc.
SAMPLE_RATE = 24000

# Lazy global — loaded once on startup.
_pipeline = None

# Common voice IDs shipped with kokoro-v1.0
KNOWN_VOICES = [
    # American English female
    "af_alloy", "af_aoede", "af_bella", "af_heart", "af_jessica", "af_kore",
    "af_nicole", "af_nova", "af_river", "af_sarah", "af_sky",
    # American English male
    "am_adam", "am_echo", "am_eric", "am_fenrir", "am_liam",
    "am_michael", "am_onyx", "am_puck", "am_santa",
    # British female
    "bf_alice", "bf_emma", "bf_isabella", "bf_lily",
    # British male
    "bm_daniel", "bm_fable", "bm_george", "bm_lewis",
]


def load_pipeline():
    """Lazy-import kokoro so the module can be inspected without it."""
    global _pipeline
    if _pipeline is not None:
        return _pipeline
    from kokoro import KPipeline  # type: ignore

    log.info("Loading Kokoro pipeline (lang=%s)…", DEFAULT_LANG)
    _pipeline = KPipeline(lang_code=DEFAULT_LANG)
    log.info("Kokoro ready (sample_rate=%d, default_voice=%s)", SAMPLE_RATE, DEFAULT_VOICE)
    return _pipeline


def samples_to_wav_b64(samples: np.ndarray, sr: int) -> str:
    buf = io.BytesIO()
    sf.write(buf, samples, sr, format="WAV", subtype="PCM_16")
    return base64.b64encode(buf.getvalue()).decode("ascii")


def synth_one(
    text: str,
    voice: str = DEFAULT_VOICE,
    speed: float = 1.0,
) -> dict:
    """Synthesize one sentence/short paragraph and return audio + word timings.

    KPipeline yields one or more chunks for very long inputs. We concatenate
    audio and stitch word timings using a per-chunk time offset so the
    returned timings are absolute within the final audio.
    """
    pipeline = load_pipeline()

    audio_chunks = []
    words: list[dict] = []
    t_offset = 0.0  # seconds

    for result in pipeline(text, voice=voice, speed=speed):
        if result.audio is None:
            continue
        # `result.audio` is a torch.Tensor (mono float32 @ 24 kHz)
        chunk = result.audio.cpu().numpy()
        audio_chunks.append(chunk)
        chunk_dur = len(chunk) / SAMPLE_RATE

        if result.tokens:
            for tok in result.tokens:
                start = getattr(tok, "start_ts", None)
                end = getattr(tok, "end_ts", None)
                if start is None or end is None:
                    continue
                tok_text = (tok.text or "").strip()
                if not tok_text:
                    continue
                # Drop pure-punctuation tokens — see _WORDY_RE comment.
                if not _WORDY_RE.search(tok_text):
                    continue
                words.append(
                    {
                        "text": tok_text,
                        "start_ms": int((float(start) + t_offset) * 1000),
                        "end_ms": int((float(end) + t_offset) * 1000),
                    }
                )

        t_offset += chunk_dur

    if not audio_chunks:
        raise RuntimeError("Kokoro returned no audio for the given text")

    audio = np.concatenate(audio_chunks).astype(np.float32)
    duration_s = len(audio) / SAMPLE_RATE

    return {
        "text": text,
        "audio_b64": samples_to_wav_b64(audio, SAMPLE_RATE),
        "sample_rate": SAMPLE_RATE,
        "duration_ms": int(duration_s * 1000),
        "words": words,
    }


# ===== HTTP =====


class TTSRequest(BaseModel):
    text: str
    voice: str = DEFAULT_VOICE
    speed: float = 1.0


@asynccontextmanager
async def lifespan(app: FastAPI):
    # Warm the model so first /tts isn't slow.
    load_pipeline()
    # Pre-warm the voice + duration head with a tiny synth so the very first
    # real request is snappy.
    try:
        synth_one("Hello.", voice=DEFAULT_VOICE)
        log.info("Warmup synth complete")
    except Exception as e:
        log.warning("Warmup failed (non-fatal): %s", e)
    yield


app = FastAPI(lifespan=lifespan)
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)


@app.get("/health")
def health():
    return {
        "ok": True,
        "default_voice": DEFAULT_VOICE,
        "sample_rate": SAMPLE_RATE,
        "model_loaded": _pipeline is not None,
    }


@app.get("/voices")
def voices():
    return {"voices": KNOWN_VOICES}


@app.post("/tts")
def tts(req: TTSRequest):
    if not req.text.strip():
        raise HTTPException(400, "Empty text")
    try:
        return synth_one(req.text, voice=req.voice, speed=req.speed)
    except Exception as e:
        log.exception("synth failed")
        raise HTTPException(500, f"Synth failed: {e}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=17891)
    parser.add_argument("--host", default="127.0.0.1")
    args = parser.parse_args()
    uvicorn.run(app, host=args.host, port=args.port, log_level="warning")


if __name__ == "__main__":
    main()
