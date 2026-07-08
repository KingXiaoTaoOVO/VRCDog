import sys
import json
import traceback
import os


def get_loopback_device(p):
    try:
        wasapi_info = p.get_host_api_info_by_type(pyaudiowpatch.paWASAPI)
        default_speaker = p.get_device_info_by_index(wasapi_info["defaultOutputDevice"])
        for loopback in p.get_loopback_device_info_generator():
            if default_speaker["name"] in loopback["name"]:
                return loopback
        # Fallback to first loopback
        for loopback in p.get_loopback_device_info_generator():
            return loopback
    except Exception as e:
        pass
    return None


def transcribe_whisper_local(audio_data, source_lang):
    """Use local Whisper (faster-whisper or openai-whisper) for offline transcription."""
    try:
        # Try faster-whisper first (much faster, lower memory)
        from faster_whisper import WhisperModel

        model_size = os.environ.get("VRCDOG_WHISPER_MODEL", "tiny")
        compute_type = os.environ.get("VRCDOG_WHISPER_COMPUTE", "int8")

        model = WhisperModel(model_size, device="cpu", compute_type=compute_type)
        segments, info = model.transcribe(audio_data, language=source_lang.split("-")[0])
        text = " ".join(seg.text for seg in segments)
        return text.strip()
    except ImportError:
        pass

    try:
        # Fallback to openai-whisper
        import whisper

        model_size = os.environ.get("VRCDOG_WHISPER_MODEL", "tiny")
        model = whisper.load_model(model_size)
        result = model.transcribe(audio_data, language=source_lang.split("-")[0])
        return result["text"].strip()
    except ImportError:
        pass

    return None


def listen_and_translate(source_lang, engine, p, loopback):
    import speech_recognition as sr

    r = sr.Recognizer()
    r.dynamic_energy_threshold = True

    print(json.dumps({"type": "status", "message": "starting", "device": loopback['name']}), flush=True)

    with sr.Microphone(device_index=loopback['index']) as source:
        # Calibrate ambient noise
        r.adjust_for_ambient_noise(source, duration=0.5)

        while True:
            try:
                # Capture audio
                audio = r.listen(source, timeout=1, phrase_time_limit=10)

                # Transcribe
                print(json.dumps({"type": "status", "message": "recognizing"}), flush=True)

                text = ""
                if engine == "local":
                    # Try local Whisper first, fallback to Sphinx/Google
                    audio_bytes = audio.get_wav_data()
                    text = transcribe_whisper_local(audio_bytes, source_lang)
                    if text:
                        print(json.dumps({"type": "result", "text": text, "engine": "whisper_local"}), flush=True)
                        continue
                    # Fallback to CMU Sphinx (offline, less accurate)
                    try:
                        import pocketsphinx
                        text = r.recognize_sphinx(audio, language=source_lang.split("-")[0])
                    except ImportError:
                        # Last resort: use Google but note engine mismatch
                        text = r.recognize_google(audio, language=source_lang)
                        if text:
                            print(json.dumps({
                                "type": "result",
                                "text": text,
                                "engine": "google_fallback",
                                "note": "Local Whisper not installed. Run: pip install faster-whisper"
                            }), flush=True)
                            continue
                else:
                    text = r.recognize_google(audio, language=source_lang)

                if text and engine != "local":
                    print(json.dumps({"type": "result", "text": text, "engine": engine}), flush=True)
            except sr.WaitTimeoutError:
                continue
            except sr.UnknownValueError:
                continue
            except Exception as e:
                print(json.dumps({"type": "error", "message": str(e)}), flush=True)

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print(json.dumps({"type": "error", "message": "Usage: vrcdog_audio.py <source_lang> <engine>"}))
        sys.exit(1)

    source_lang = sys.argv[1]
    engine = sys.argv[2]

    try:
        import pyaudiowpatch as pyaudio
        sys.modules['pyaudio'] = pyaudio
        import speech_recognition as sr
    except ImportError as e:
        print(json.dumps({
            "type": "error",
            "message": f"Dependency missing: {e}. Run: pip install pyaudiowpatch SpeechRecognition"
        }), flush=True)
        sys.exit(1)

    with pyaudio.PyAudio() as p:
        loopback = get_loopback_device(p)
        if not loopback:
            print(json.dumps({"type": "error", "message": "No WASAPI loopback device found"}), flush=True)
            sys.exit(1)

        try:
            listen_and_translate(source_lang, engine, p, loopback)
        except KeyboardInterrupt:
            sys.exit(0)
        except Exception as e:
            print(json.dumps({"type": "error", "message": traceback.format_exc()}), flush=True)
