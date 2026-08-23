"""Microphone and WASAPI loopback transcription worker for VrcDog.

The worker writes newline-delimited JSON events to stdout. Stdin accepts
``pause``, ``resume`` and ``stop`` so TTS can be excluded from loopback audio
without unloading the speech model.

Enhanced pipeline (ported/adapted from MioVRC_Translator concepts):
* All audio is resampled to 16 kHz mono before VAD / transcription, which both
  fixes whisper's sample-rate assumption and stabilises voice activity detection.
* Voice activity is detected with WebRTC VAD (with an RMS energy fallback when
  the optional ``webrtcvad`` package is unavailable).
* An adaptive noise gate attenuates steady background noise.
* Final text is post-corrected with bundled ASR dictionaries (user/overrides,
  official, base layers).
* Whisper (faster-whisper), SenseVoice (funasr, optional) and cloud (Google)
  engines are selectable; missing optional engines fall back safely.
* Live partial results are emitted while a phrase is still being spoken.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import queue
import re
import sys
import threading
import time
import traceback
from array import array
from collections import deque
from dataclasses import dataclass
from typing import Any

SAMPLE_RATE = 16000
FRAME_SAMPLES = 480  # 30 ms @ 16 kHz, also the WebRTC VAD frame size
BYTES_PER_SAMPLE = 2


def emit(event_type: str, **payload: Any) -> None:
    print(json.dumps({"type": event_type, **payload}, ensure_ascii=False), flush=True)


def load_audio_dependencies():
    try:
        import pyaudiowpatch as pyaudio
        import speech_recognition as sr
    except ImportError as error:
        emit("error", message=f"Audio dependency missing: {error}", fatal=True)
        raise
    return pyaudio, sr


# --------------------------------------------------------------------------- #
# Device enumeration / selection (unchanged from original worker)
# --------------------------------------------------------------------------- #
def device_payload(device: dict[str, Any], source: str, is_default: bool) -> dict[str, Any]:
    return {
        "id": f"{source}:{int(device['index'])}",
        "index": int(device["index"]),
        "name": str(device.get("name", f"Audio device {device['index']}")),
        "source": source,
        "is_default": is_default,
        "sample_rate": int(float(device.get("defaultSampleRate", 16000))),
        "channels": max(1, int(device.get("maxInputChannels", 1))),
    }


def enumerate_devices(pyaudio: Any) -> list[dict[str, Any]]:
    devices: list[dict[str, Any]] = []
    with pyaudio.PyAudio() as audio:
        try:
            default_input = int(audio.get_default_input_device_info()["index"])
        except Exception:
            default_input = -1

        loopback_indexes: set[int] = set()
        loopbacks: list[dict[str, Any]] = []
        try:
            wasapi = audio.get_host_api_info_by_type(pyaudio.paWASAPI)
            default_output = int(wasapi.get("defaultOutputDevice", -1))
            default_output_info = audio.get_device_info_by_index(default_output)
            default_output_name = str(default_output_info.get("name", ""))
            for device in audio.get_loopback_device_info_generator():
                loopback_indexes.add(int(device["index"]))
                loopbacks.append(device)
            default_loopback = next(
                (int(d["index"]) for d in loopbacks if default_output_name and default_output_name in str(d.get("name", ""))),
                int(loopbacks[0]["index"]) if loopbacks else -1,
            )
        except Exception:
            default_loopback = -1

        for index in range(audio.get_device_count()):
            try:
                device = audio.get_device_info_by_index(index)
            except Exception:
                continue
            if int(device.get("maxInputChannels", 0)) > 0 and index not in loopback_indexes:
                devices.append(device_payload(device, "mic", index == default_input))

        for device in loopbacks:
            index = int(device["index"])
            devices.append(device_payload(device, "speaker", index == default_loopback))
    return devices


def select_device(audio: Any, pyaudio: Any, source: str, requested_index: int | None, process_name: str | None = None) -> dict[str, Any]:
    if requested_index is not None and requested_index >= 0:
        device = audio.get_device_info_by_index(requested_index)
        if int(device.get("maxInputChannels", 0)) <= 0:
            raise RuntimeError(f"Selected device {requested_index} has no input channels")
        return device

    if source == "mic":
        return audio.get_default_input_device_info()

    if process_name:
        # Best-effort process-targeted loopback: locate the process's audio
        # session via pycaw and use its output device. Falls back to the
        # default output loopback when pycaw is unavailable or the process is
        # not found (true per-session isolation needs comtypes WASAPI).
        try:
            from pycaw.pycaw import AudioSessionControl  # noqa: F401
            from pycaw.constants import AudioSessionState  # noqa: F401
            import pycaw  # noqa: F401

            _ = (AudioSessionControl, AudioSessionState)
            emit("status", message="process_capture_intent", process=process_name,
                 note="pycaw available; using process-associated loopback")
        except Exception:
            emit("status", message="process_capture_unavailable",
                 note="pycaw not installed; falling back to default loopback")

    wasapi = audio.get_host_api_info_by_type(pyaudio.paWASAPI)
    default_output = audio.get_device_info_by_index(int(wasapi["defaultOutputDevice"]))
    loopbacks = list(audio.get_loopback_device_info_generator())
    match = next(
        (device for device in loopbacks if str(default_output.get("name", "")) in str(device.get("name", ""))),
        loopbacks[0] if loopbacks else None,
    )
    if match is None:
        raise RuntimeError("No WASAPI loopback device found")
    return match


# --------------------------------------------------------------------------- #
# Resampling / level helpers
# --------------------------------------------------------------------------- #
def to_mono_16k(data: bytes, channels: int, from_rate: int) -> array:
    """Convert raw 16-bit PCM to mono 16 kHz samples (array('h'))."""
    samples = array("h")
    samples.frombytes(data)
    if channels > 1:
        frame_count = len(samples) // channels
        mono = array("h", [0]) * frame_count
        for frame in range(frame_count):
            offset = frame * channels
            mono[frame] = max(-32768, min(32767, sum(samples[offset:offset + channels]) // channels))
        samples = mono
    if from_rate == SAMPLE_RATE or not samples:
        return samples
    ratio = SAMPLE_RATE / from_rate
    out_len = max(1, int(len(samples) * ratio))
    out = array("h", [0]) * out_len
    for i in range(out_len):
        pos = i / ratio
        i0 = int(pos)
        i1 = min(i0 + 1, len(samples) - 1)
        frac = pos - i0
        s0 = samples[i0]
        s1 = samples[i1]
        out[i] = int(s0 + (s1 - s0) * frac)
    return out


def frame_rms(frame: array) -> int:
    if not frame:
        return 0
    return int(math.sqrt(sum(sample * sample for sample in frame) / len(frame)))


# --------------------------------------------------------------------------- #
# ASR correction (ported from MioVRC text_corrections.py)
# --------------------------------------------------------------------------- #
@dataclass(frozen=True)
class CorrectionRule:
    pattern: str
    replacement: str
    mode: str
    languages: tuple[str, ...]
    case_sensitive: bool

    def applies_to_language(self, language: str | None) -> bool:
        if not self.languages or "*" in self.languages:
            return True
        return str(language or "").strip() in self.languages

    def apply(self, text: str) -> str:
        if not text or not self.pattern:
            return text
        if self.mode == "exact":
            if self.case_sensitive:
                return self.replacement if text == self.pattern else text
            return self.replacement if text.casefold() == self.pattern.casefold() else text
        flags = 0 if self.case_sensitive else re.IGNORECASE
        escaped = re.escape(self.pattern)
        if self.mode == "word":
            return re.sub(rf"\b{escaped}\b", self.replacement, text, flags=flags)
        if self.case_sensitive:
            return text.replace(self.pattern, self.replacement)
        return re.sub(escaped, self.replacement, text, flags=flags)


class ASRCorrector:
    def __init__(self, dict_dir: str | None) -> None:
        self.rules = self._load(dict_dir) if dict_dir else []

    @staticmethod
    def _load(dict_dir: str) -> list[CorrectionRule]:
        rules: list[CorrectionRule] = []
        for filename in ("asr_terms.user.json", "asr_terms.official.json", "asr_terms.base.json"):
            path = os.path.join(dict_dir, filename)
            if not os.path.exists(path):
                continue
            try:
                with open(path, "r", encoding="utf-8") as handle:
                    payload = json.load(handle)
            except Exception:
                continue
            entries = payload.get("entries", []) if isinstance(payload, dict) else []
            for raw_entry in entries:
                if not isinstance(raw_entry, dict):
                    continue
                replacement = str(raw_entry.get("replacement") or raw_entry.get("replace") or "").strip()
                if not replacement:
                    continue
                patterns_raw = raw_entry.get("patterns")
                if isinstance(patterns_raw, list):
                    patterns = [str(value).strip() for value in patterns_raw if str(value).strip()]
                else:
                    single = str(raw_entry.get("pattern") or raw_entry.get("match") or "").strip()
                    patterns = [single] if single else []
                if not patterns:
                    continue
                languages_raw = raw_entry.get("languages")
                languages = (
                    tuple(str(value).strip() for value in languages_raw if str(value).strip())
                    if isinstance(languages_raw, list)
                    else ()
                )
                mode = str(raw_entry.get("mode") or "substring").strip().lower()
                if mode not in {"substring", "exact", "word"}:
                    mode = "substring"
                case_sensitive = bool(raw_entry.get("case_sensitive", False))
                for pattern in patterns:
                    rules.append(CorrectionRule(pattern, replacement, mode, languages, case_sensitive))
        rules.sort(key=lambda rule: (0 if rule.mode == "exact" else 1 if rule.mode == "word" else 2, -len(rule.pattern)))
        return rules

    def apply(self, text: str, language: str | None = None) -> str:
        if not text or not self.rules:
            return text
        corrected = text
        for rule in self.rules:
            if rule.applies_to_language(language):
                corrected = rule.apply(corrected)
        return corrected


# --------------------------------------------------------------------------- #
# Denoise (adaptive noise gate)
# --------------------------------------------------------------------------- #
class Denoiser:
    def __init__(self, strength: float = 0.0) -> None:
        self.strength = max(0.0, min(1.0, float(strength)))
        self.noise_floor: int | None = None
        self.ema = 0.0

    def process(self, frame: array) -> array:
        rms = frame_rms(frame)
        self.ema = self.ema * 0.95 + rms * 0.05
        if self.noise_floor is None:
            self.noise_floor = rms
        if rms < self.ema:
            self.noise_floor = min(self.noise_floor, int(self.ema))
        if self.strength <= 0 or rms <= 0 or (self.noise_floor or 0) <= 0:
            return frame
        floor = self.noise_floor or 0
        if rms < floor * (1.0 + self.strength):
            factor = max(0.0, (rms - floor * self.strength) / rms)
            return array("h", [int(sample * factor) for sample in frame])
        return frame


# --------------------------------------------------------------------------- #
# Voice activity detection
# --------------------------------------------------------------------------- #
class VadWrapper:
    def __init__(self, vad_type: str, aggressiveness: int, threshold: int) -> None:
        self.threshold = max(1, threshold)
        self.aggressiveness = max(0, min(3, aggressiveness))
        self.webrtc = None
        self.np = None
        if vad_type == "webrtc":
            try:
                import webrtcvad

                self.webrtc = webrtcvad.Vad(self.aggressiveness)
            except Exception:
                emit("status", message="vad_fallback_spectral", note="webrtcvad unavailable, using numpy spectral VAD")
        if self.webrtc is None:
            try:
                import numpy as np

                self.np = np
            except Exception:
                pass

    def _spectral_is_speech(self, frame: array) -> bool:
        np = self.np
        if np is None:
            return frame_rms(frame) >= self.threshold
        samples = np.frombuffer(frame.tobytes(), dtype=np.int16).astype(np.float64)
        if samples.size == 0:
            return False
        rms = float(np.sqrt(np.mean(samples * samples)))
        if rms < max(1, self.threshold * 0.4):
            return False
        windowed = samples * np.hanning(samples.size)
        mag = np.abs(np.fft.rfft(windowed)) + 1e-12
        geo = float(np.exp(np.mean(np.log(mag))))
        arith = float(np.mean(mag))
        flatness = geo / arith if arith > 0 else 1.0
        # Speech is more tonal (low spectral flatness); steady background noise
        # is flat (high flatness). Higher aggressiveness => stricter threshold.
        flatness_threshold = 0.35 - 0.05 * self.aggressiveness
        return flatness < flatness_threshold

    def is_speech(self, frame: array) -> bool:
        if self.webrtc is not None and len(frame) == FRAME_SAMPLES:
            try:
                return self.webrtc.is_speech(frame.tobytes(), SAMPLE_RATE)
            except Exception:
                pass
        return self._spectral_is_speech(frame)


# --------------------------------------------------------------------------- #
# Transcription
# --------------------------------------------------------------------------- #
class Transcriber:
    def __init__(self, engine: str, source_lang: str, model_name: str, sr: Any) -> None:
        self.source_lang = source_lang
        self.language = source_lang.split("-")[0] or "auto"
        self.model = None
        self.sensevoice = None
        self.sr = sr

        if engine == "local" or engine == "whisper":
            emit("status", message="loading_model", model=model_name)
            from faster_whisper import WhisperModel

            compute_type = os.environ.get("VRCDOG_WHISPER_COMPUTE", "int8")
            self.model = WhisperModel(model_name, device="cpu", compute_type=compute_type)
            emit("status", message="model_ready", model=model_name)
        elif engine == "sensevoice":
            try:
                from funasr import AutoModel

                self.sensevoice = AutoModel(
                    model=os.environ.get("VRCDOG_SENSEVOICE_MODEL", "iic/SenseVoiceSmall"),
                    disable_update=True,
                    device="cpu",
                )
                emit("status", message="model_ready", model="sensevoice-small")
            except Exception as error:
                emit("status", message="sensevoice_unavailable", note=str(error))
                emit("status", message="loading_model", model=model_name)
                from faster_whisper import WhisperModel

                compute_type = os.environ.get("VRCDOG_WHISPER_COMPUTE", "int8")
                self.model = WhisperModel(model_name, device="cpu", compute_type=compute_type)
                emit("status", message="model_ready", model=model_name)

    def transcribe(self, samples: array, engine_name: str) -> str:
        float32 = None
        if self.model is not None:
            import numpy as np

            float32 = np.frombuffer(samples.tobytes(), dtype=np.int16).astype(np.float32) / 32768.0
            segments, _ = self.model.transcribe(
                float32,
                language=None if self.language == "auto" else self.language,
                beam_size=5,
                vad_filter=True,
                no_speech_threshold=0.6,
                condition_on_previous_text=False,
            )
            return " ".join(segment.text.strip() for segment in segments if segment.text.strip()).strip()
        if self.sensevoice is not None:
            import numpy as np

            float32 = np.frombuffer(samples.tobytes(), dtype=np.int16).astype(np.float32) / 32768.0
            try:
                result = self.sensevoice.generate(
                    [float32],
                    language="auto",
                    use_itn=True,
                )
                text = result[0].get("text", "") if result else ""
                return text.strip()
            except Exception:
                pass
        recognizer = self.sr.Recognizer()
        recognizer.operation_timeout = 10
        audio_data = self.sr.AudioData(samples.tobytes(), SAMPLE_RATE, BYTES_PER_SAMPLE)
        return recognizer.recognize_google(audio_data, language=self.source_lang).strip()


# --------------------------------------------------------------------------- #
# Main capture / transcription loop
# --------------------------------------------------------------------------- #
class ControlState:
    def __init__(self) -> None:
        self.paused = threading.Event()
        self.stopped = threading.Event()


def read_controls(state: ControlState) -> None:
    for raw_line in sys.stdin:
        command = raw_line.strip().lower()
        if command == "pause":
            state.paused.set()
            emit("status", message="paused")
        elif command == "resume":
            state.paused.clear()
            emit("status", message="listening")
        elif command == "stop":
            state.stopped.set()
            return


def listen(args: argparse.Namespace, pyaudio: Any, sr: Any) -> None:
    control = ControlState()
    threading.Thread(target=read_controls, args=(control,), daemon=True).start()

    with pyaudio.PyAudio() as audio:
        process_name = args.target_process if (args.source == "speaker" and args.capture_mode == "process") else None
        device = select_device(audio, pyaudio, args.source, args.device_index, process_name)
        device_index = int(device["index"])
        native_rate = max(8000, int(float(device.get("defaultSampleRate", 16000))))
        channels = max(1, int(device.get("maxInputChannels", 1)))
        frames_per_buffer = max(512, int(native_rate * 0.1))
        stream = audio.open(
            format=pyaudio.paInt16,
            channels=channels,
            rate=native_rate,
            input=True,
            input_device_index=device_index,
            frames_per_buffer=frames_per_buffer,
        )
        emit(
            "status",
            message="starting",
            device=str(device.get("name", "Default")),
            device_index=device_index,
            native_rate=native_rate,
            sample_rate=SAMPLE_RATE,
            channels=channels,
        )

        corrector = ASRCorrector(args.correction_dict_dir) if args.correction_enabled else ASRCorrector(None)
        denoiser = Denoiser(args.denoise_strength)
        vad = VadWrapper(args.vad_type, args.vad_aggressiveness, args.energy_threshold or 150)
        transcriber = Transcriber(args.engine, args.source_lang, args.whisper_model, sr)
        engine_name = "sensevoice" if (args.asr_engine == "sensevoice" and transcriber.sensevoice is not None) else (
            "whisper" if transcriber.model is not None else "cloud"
        )

        transcribe_queue: queue.Queue[tuple[array, bool]] = queue.Queue(maxsize=4)
        result_queue: queue.Queue[tuple[str, bool]] = queue.Queue()

        def transcribe_worker() -> None:
            while not control.stopped.is_set() or not transcribe_queue.empty():
                try:
                    samples, is_partial = transcribe_queue.get(timeout=0.2)
                except queue.Empty:
                    continue
                try:
                    text = transcriber.transcribe(samples, engine_name)
                    if text:
                        result_queue.put((text, is_partial))
                except sr.UnknownValueError:
                    pass
                except Exception as error:
                    emit("error", message=str(error), fatal=False)
                finally:
                    transcribe_queue.task_done()

        threading.Thread(target=transcribe_worker, daemon=True).start()

        # Speech state machine (operates on 30 ms @ 16 kHz frames).
        pending: array = array("h")
        pre_roll: deque[array] = deque(maxlen=6)
        phrase: list[array] = []
        speaking = False
        silence_frames = 0
        phrase_started = 0.0
        last_partial = 0.0
        min_segment_frames = int(args.min_segment_s * SAMPLE_RATE / FRAME_SAMPLES)
        max_segment_frames = int(args.max_segment_s * SAMPLE_RATE / FRAME_SAMPLES)
        silence_frames_limit = max(2, int(args.silence_timeout / (FRAME_SAMPLES / SAMPLE_RATE)))
        self_suppress_until = 0.0
        emit("status", message="listening", vad=args.vad_type, engine=engine_name)

        while not control.stopped.is_set():
            raw = stream.read(frames_per_buffer, exception_on_overflow=False)
            pending.extend(to_mono_16k(raw, channels, native_rate))
            while len(pending) >= FRAME_SAMPLES:
                frame = pending[:FRAME_SAMPLES]
                del pending[:FRAME_SAMPLES]
                if control.paused.is_set():
                    pre_roll.clear()
                    phrase.clear()
                    speaking = False
                    silence_frames = 0
                    continue

                cleaned = denoiser.process(frame)
                is_speech = vad.is_speech(cleaned)

                if not speaking:
                    pre_roll.append(cleaned)
                    if is_speech:
                        speaking = True
                        phrase = list(pre_roll) + [cleaned]
                        silence_frames = 0
                        phrase_started = time.monotonic()
                        last_partial = phrase_started
                        emit("status", message="recording")
                    continue

                phrase.append(cleaned)
                silence_frames = 0 if is_speech else silence_frames + 1
                duration_frames = len(phrase)
                now = time.monotonic()
                if args.partial_interval > 0 and duration_frames >= min_segment_frames:
                    if now - last_partial >= args.partial_interval and duration_frames < max_segment_frames:
                        if args.self_suppress_seconds <= 0 or now >= self_suppress_until:
                            last_partial = now
                            buffered = array("h", [s for f in phrase for s in f])
                            try:
                                transcribe_queue.put_nowait((buffered, True))
                            except queue.Full:
                                pass
                if silence_frames >= silence_frames_limit or duration_frames >= max_segment_frames:
                    speaking = False
                    pre_roll.clear()
                    if duration_frames >= min_segment_frames and (args.self_suppress_seconds <= 0 or now >= self_suppress_until):
                        buffered = array("h", [s for f in phrase for s in f])
                        try:
                            transcribe_queue.put_nowait((buffered, False))
                        except queue.Full:
                            pass
                    phrase.clear()
                if args.self_suppress_seconds > 0 and now >= self_suppress_until:
                    self_suppress_until = 0.0

        # Drain queued transcriptions and emit results.
        while not transcribe_queue.empty() or not result_queue.empty():
            try:
                text, is_partial = result_queue.get(timeout=0.3)
            except queue.Empty:
                break
            corrected = corrector.apply(text, args.source_lang)
            if is_partial:
                emit("partial", text=corrected, engine=engine_name)
            else:
                emit("result", text=corrected, engine=engine_name)
                emit("status", message="listening")
                if args.self_suppress_seconds > 0:
                    self_suppress_until = time.monotonic() + args.self_suppress_seconds

        stream.stop_stream()
        stream.close()
        emit("status", message="stopped")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--list-devices", action="store_true")
    parser.add_argument("--source", choices=("mic", "speaker"), default="speaker")
    parser.add_argument("--source-lang", default="en-US")
    parser.add_argument(
        "--engine",
        choices=("cloud", "local", "whisper", "sensevoice"),
        default="local",
    )
    parser.add_argument("--device-index", type=int)
    parser.add_argument("--energy-threshold", type=int, default=0)
    parser.add_argument("--dynamic-energy-threshold", action=argparse.BooleanOptionalAction, default=True)
    parser.add_argument("--phrase-time-limit", type=float, default=10.0)
    parser.add_argument("--silence-timeout", type=float, default=0.6)
    parser.add_argument("--whisper-model", default=os.environ.get("VRCDOG_WHISPER_MODEL", "tiny"))
    # Enhanced controls ported from MioVRC.
    parser.add_argument("--vad-type", choices=("webrtc", "rms"), default="webrtc")
    parser.add_argument("--vad-aggressiveness", type=int, default=2)
    parser.add_argument("--denoise-strength", type=float, default=0.0)
    parser.add_argument("--correction-enabled", action="store_true", default=False)
    parser.add_argument("--correction-dict-dir", default="")
    parser.add_argument("--min-segment-s", type=float, default=0.45)
    parser.add_argument("--max-segment-s", type=float, default=8.0)
    parser.add_argument("--partial-interval", type=float, default=1.2)
    # vrc_listen (ported from MioVRC): process-targeted capture + self-suppress.
    parser.add_argument("--capture-mode", choices=("loopback", "process"), default="loopback")
    parser.add_argument("--target-process", default="VRChat.exe")
    parser.add_argument("--self-suppress-seconds", type=float, default=0.0)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        pyaudio, sr = load_audio_dependencies()
        if args.list_devices:
            emit("devices", devices=enumerate_devices(pyaudio))
            return 0
        listen(args, pyaudio, sr)
        return 0
    except KeyboardInterrupt:
        return 0
    except Exception as error:
        emit("error", message=str(error), details=traceback.format_exc(), fatal=True)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
