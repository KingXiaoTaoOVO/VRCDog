"""Microphone and WASAPI loopback transcription worker for VrcDog.

The worker writes newline-delimited JSON events to stdout. Stdin accepts
``pause``, ``resume`` and ``stop`` so TTS can be excluded from loopback audio
without unloading the speech model.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import queue
import sys
import threading
import time
import traceback
from array import array
from collections import deque
from typing import Any


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


def select_device(audio: Any, pyaudio: Any, source: str, requested_index: int | None) -> dict[str, Any]:
    if requested_index is not None and requested_index >= 0:
        device = audio.get_device_info_by_index(requested_index)
        if int(device.get("maxInputChannels", 0)) <= 0:
            raise RuntimeError(f"Selected device {requested_index} has no input channels")
        return device

    if source == "mic":
        return audio.get_default_input_device_info()

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


def pcm_to_mono(data: bytes, channels: int) -> bytes:
    if channels <= 1:
        return data
    samples = array("h")
    samples.frombytes(data)
    frame_count = len(samples) // channels
    mono = array("h", [0]) * frame_count
    for frame in range(frame_count):
        offset = frame * channels
        mono[frame] = max(-32768, min(32767, sum(samples[offset:offset + channels]) // channels))
    return mono.tobytes()


def rms_level(pcm: bytes) -> int:
    samples = array("h")
    samples.frombytes(pcm)
    if not samples:
        return 0
    return int(math.sqrt(sum(sample * sample for sample in samples) / len(samples)))


class Transcriber:
    def __init__(self, engine: str, source_lang: str, model_name: str, sr: Any) -> None:
        self.source_lang = source_lang
        self.language = source_lang.split("-")[0]
        self.model = None
        self.sr = sr
        if engine == "local":
            emit("status", message="loading_model", model=model_name)
            from faster_whisper import WhisperModel

            compute_type = os.environ.get("VRCDOG_WHISPER_COMPUTE", "int8")
            self.model = WhisperModel(model_name, device="cpu", compute_type=compute_type)
            emit("status", message="model_ready", model=model_name)

    def transcribe(self, pcm: bytes, sample_rate: int) -> str:
        if self.model is not None:
            import numpy as np

            samples = np.frombuffer(pcm, dtype=np.int16).astype(np.float32) / 32768.0
            segments, _ = self.model.transcribe(
                samples,
                language=self.language,
                beam_size=5,
                vad_filter=True,
                no_speech_threshold=0.6,
                condition_on_previous_text=False,
            )
            return " ".join(segment.text.strip() for segment in segments if segment.text.strip()).strip()

        recognizer = self.sr.Recognizer()
        recognizer.operation_timeout = 10
        audio_data = self.sr.AudioData(pcm, sample_rate, 2)
        return recognizer.recognize_google(audio_data, language=self.source_lang).strip()


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


def transcribe_queued_audio(
    transcriber: Transcriber,
    sr: Any,
    audio_queue: queue.Queue[bytes],
    sample_rate: int,
    stopped: threading.Event,
) -> None:
    while not stopped.is_set() or not audio_queue.empty():
        try:
            pcm = audio_queue.get(timeout=0.2)
        except queue.Empty:
            continue
        emit("status", message="recognizing", queued=audio_queue.qsize())
        try:
            text = transcriber.transcribe(pcm, sample_rate)
            if text:
                emit("result", text=text, engine="local" if transcriber.model is not None else "cloud")
        except sr.UnknownValueError:
            pass
        except Exception as error:
            emit("error", message=str(error), fatal=False)
        finally:
            audio_queue.task_done()
        emit("status", message="listening")


def listen(args: argparse.Namespace, pyaudio: Any, sr: Any) -> None:
    control = ControlState()
    threading.Thread(target=read_controls, args=(control,), daemon=True).start()

    with pyaudio.PyAudio() as audio:
        device = select_device(audio, pyaudio, args.source, args.device_index)
        device_index = int(device["index"])
        sample_rate = max(8000, int(float(device.get("defaultSampleRate", 16000))))
        channels = max(1, int(device.get("maxInputChannels", 1)))
        frames_per_buffer = max(512, int(sample_rate * 0.1))
        stream = audio.open(
            format=pyaudio.paInt16,
            channels=channels,
            rate=sample_rate,
            input=True,
            input_device_index=device_index,
            frames_per_buffer=frames_per_buffer,
        )
        emit(
            "status",
            message="starting",
            device=str(device.get("name", "Default")),
            device_index=device_index,
            sample_rate=sample_rate,
            channels=channels,
        )

        transcriber = Transcriber(args.engine, args.source_lang, args.whisper_model, sr)
        pending_audio: queue.Queue[bytes] = queue.Queue(maxsize=4)
        transcriber_thread = threading.Thread(
            target=transcribe_queued_audio,
            args=(transcriber, sr, pending_audio, sample_rate, control.stopped),
            daemon=True,
        )
        transcriber_thread.start()
        threshold = args.energy_threshold
        emit("status", message="listening", threshold=threshold, calibrated=threshold > 0)

        calibration_levels: list[int] = []
        for _ in range(5):
            raw = stream.read(frames_per_buffer, exception_on_overflow=False)
            calibration_levels.append(rms_level(pcm_to_mono(raw, channels)))
        ambient = sum(calibration_levels) // max(1, len(calibration_levels))
        threshold = args.energy_threshold if args.energy_threshold > 0 else max(120, int(ambient * 1.8))
        emit("status", message="listening", threshold=threshold, calibrated=True)
        pre_roll: deque[bytes] = deque(maxlen=3)
        phrase: list[bytes] = []
        speaking = False
        silence_chunks = 0
        phrase_started = 0.0
        silence_limit = max(3, int(args.silence_timeout / 0.1))

        while not control.stopped.is_set():
            raw = stream.read(frames_per_buffer, exception_on_overflow=False)
            mono = pcm_to_mono(raw, channels)
            if control.paused.is_set():
                pre_roll.clear()
                phrase.clear()
                speaking = False
                silence_chunks = 0
                continue

            level = rms_level(mono)
            if args.dynamic_energy_threshold and not speaking:
                target = max(100, int(level * 1.6))
                threshold = int(threshold * 0.97 + target * 0.03)

            if not speaking:
                pre_roll.append(mono)
                if level >= threshold:
                    speaking = True
                    phrase = list(pre_roll)
                    silence_chunks = 0
                    phrase_started = time.monotonic()
                    emit("status", message="recording", level=level, threshold=threshold)
                continue

            phrase.append(mono)
            silence_chunks = silence_chunks + 1 if level < threshold else 0
            duration = time.monotonic() - phrase_started
            if silence_chunks < silence_limit and duration < args.phrase_time_limit:
                continue

            speaking = False
            pre_roll.clear()
            if duration < 0.25:
                phrase.clear()
                continue

            pcm = b"".join(phrase)
            phrase.clear()
            try:
                pending_audio.put_nowait(pcm)
            except queue.Full:
                try:
                    pending_audio.get_nowait()
                    pending_audio.task_done()
                except queue.Empty:
                    pass
                pending_audio.put_nowait(pcm)
                emit("status", message="backlog_trimmed", queued=pending_audio.qsize())

        stream.stop_stream()
        stream.close()
        transcriber_thread.join(timeout=2)
        emit("status", message="stopped")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--list-devices", action="store_true")
    parser.add_argument("--source", choices=("mic", "speaker"), default="speaker")
    parser.add_argument("--source-lang", default="en-US")
    parser.add_argument("--engine", choices=("cloud", "local"), default="cloud")
    parser.add_argument("--device-index", type=int)
    parser.add_argument("--energy-threshold", type=int, default=0)
    parser.add_argument("--dynamic-energy-threshold", action=argparse.BooleanOptionalAction, default=True)
    parser.add_argument("--phrase-time-limit", type=float, default=10.0)
    parser.add_argument("--silence-timeout", type=float, default=0.8)
    parser.add_argument("--whisper-model", default=os.environ.get("VRCDOG_WHISPER_MODEL", "tiny"))
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
