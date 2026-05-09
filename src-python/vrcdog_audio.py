import sys
import json
import traceback

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

def listen_and_translate(source_lang, engine, p, loopback):
    import speech_recognition as sr
    r = sr.Recognizer()
    r.dynamic_energy_threshold = True
    
    print(json.dumps({"type": "status", "message": "starting", "device": loopback['name']}), flush=True)

    with sr.Microphone(device_index=loopback['index']) as source:
        while True:
            try:
                # Capture audio
                audio = r.listen(source, timeout=1, phrase_time_limit=10)
                
                # Transcribe
                print(json.dumps({"type": "status", "message": "recognizing"}), flush=True)
                
                text = ""
                if engine == "cloud":
                    text = r.recognize_google(audio, language=source_lang)
                else:
                    # Mock Whisper or implement it if installed
                    text = r.recognize_google(audio, language=source_lang) 
                    
                if text:
                    print(json.dumps({"type": "result", "text": text}), flush=True)
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
        print(json.dumps({"type": "error", "message": f"Dependency missing: {e}. Run pip install pyaudiowpatch SpeechRecognition"}), flush=True)
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
