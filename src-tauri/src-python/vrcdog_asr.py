"""VRCDog ASR bridge - transcribe audio using faster-whisper."""
import argparse
import json
import sys
import os

def main():
    parser = argparse.ArgumentParser(description="VRCDog ASR bridge")
    parser.add_argument("audio_path", help="Path to audio file (WAV)")
    parser.add_argument("--model", default="small", help="Whisper model size")
    # 默认必须由 None 走 Whisper 自动识别：原先写死 "zh"，而 Rust 侧只传音频路径，
    # 于是任何语言的语音输入都被强制按中文解码，英文/日文语音转出来是一堆乱码。
    parser.add_argument("--language", default=None, help="Language code (default: auto-detect)")
    parser.add_argument("--device", default="cpu", help="Device: cpu or cuda")
    args = parser.parse_args()

    try:
        from faster_whisper import WhisperModel
    except ImportError:
        print(json.dumps({"error": "faster-whisper not installed"}))
        sys.exit(1)

    audio_path = args.audio_path
    if not os.path.exists(audio_path):
        print(json.dumps({"error": f"Audio file not found: {audio_path}"}))
        sys.exit(1)

    try:
        model = WhisperModel(args.model, device=args.device, compute_type="int8")
        segments, info = model.transcribe(
            audio_path,
            language=args.language,
            beam_size=5,
            vad_filter=True,
            vad_parameters=dict(min_silence_duration_ms=500),
        )

        text_parts = []
        for segment in segments:
            text_parts.append(segment.text.strip())

        transcription = " ".join(text_parts).strip()
        result = {
            "text": transcription,
            "language": info.language,
            "duration": info.duration,
        }
        print(json.dumps(result, ensure_ascii=False))
    except Exception as e:
        print(json.dumps({"error": str(e)}))
        sys.exit(1)

if __name__ == "__main__":
    main()
