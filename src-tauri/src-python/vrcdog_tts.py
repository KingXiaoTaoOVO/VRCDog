"""VRCDog TTS bridge - synthesize speech using edge-tts."""
import argparse
import asyncio
import json
import os
import sys

async def synthesize_async(text, voice, rate, volume, output_path):
    import edge_tts
    communicate = edge_tts.Communicate(text, voice, rate=rate, volume=volume)
    await communicate.save(output_path)
    return output_path

def main():
    parser = argparse.ArgumentParser(description="VRCDog TTS bridge")
    parser.add_argument("--text", required=True, help="Text to synthesize")
    parser.add_argument("--voice", default="zh-CN-XiaoxiaoNeural", help="Edge TTS voice ID")
    parser.add_argument("--rate", default="+0%", help="Speech rate adjustment (e.g. +0%%, +50%%)")
    parser.add_argument("--volume", default="+0%", help="Volume adjustment (e.g. +0%%, +50%%)")
    parser.add_argument("--output", required=True, help="Output audio file path")
    args = parser.parse_args()

    if not args.text or not args.text.strip():
        print(json.dumps({"error": "Text cannot be empty"}))
        sys.exit(1)

    try:
        output_path = asyncio.run(synthesize_async(
            args.text.strip(),
            args.voice,
            args.rate,
            args.volume,
            args.output,
        ))
        result = {
            "output_path": output_path,
            "voice": args.voice,
            "text": args.text,
        }
        print(json.dumps(result, ensure_ascii=False))
    except Exception as e:
        print(json.dumps({"error": str(e)}))
        sys.exit(1)

if __name__ == "__main__":
    main()
