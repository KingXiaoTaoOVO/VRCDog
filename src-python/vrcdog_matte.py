#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
VRCDog 前景抠图（U²-Netp ONNX）

输入任意图片，输出一张与原图同尺寸的灰度 alpha 遮罩 PNG（白=前景主体，黑=背景）。
Rust 侧把它二值化后当作前景掩码使用，因此这里只关心「主体在哪」，不做去色边缘细化。

模型权重不打包进安装包：由 `model_download_matting` 按需下载到 app_data/models，
带 SHA-256 校验。下载失败或文件损坏时 Rust 会退回原来的启发式抠图。
"""

import argparse
import json
import os
import sys
import time
from pathlib import Path

import numpy as np

os.environ.setdefault("ORT_LOG_LEVEL", "3")

try:
    import onnxruntime as ort
    from PIL import Image
except ImportError as error:  # 依赖缺失必须让调用方拿到结构化错误，而不是栈回溯
    print(json.dumps({"status": "error", "message": f"Missing python dependencies: {error}"}))
    sys.exit(1)

# 实测签名（.scratch 探针，2026-11）：
#   input  "input.1"  float [1,3,320,320]
#   output 7 个 [1,1,320,320]，第 0 个是融合后的 d0
MODEL_INPUT_NAME = "input.1"
MODEL_SIZE = 320
MEAN = np.array([0.485, 0.456, 0.406], dtype=np.float32).reshape(3, 1, 1)
STD = np.array([0.229, 0.224, 0.225], dtype=np.float32).reshape(3, 1, 1)


def emit(payload: dict, code: int = 0) -> None:
    print(json.dumps(payload))
    sys.exit(code)


def main() -> None:
    parser = argparse.ArgumentParser(description="VRCDog U2-Netp matting bridge")
    parser.add_argument("--input", "-i", required=True)
    parser.add_argument("--output", "-o", required=True)
    parser.add_argument("--model", "-m", required=True, help="u2netp.onnx 的绝对路径")
    parser.add_argument("--size", type=int, default=MODEL_SIZE, help="推理边长（权重是静态 320）")
    args = parser.parse_args()

    source = Path(args.input)
    model = Path(args.model)
    if not source.is_file():
        emit({"status": "error", "message": f"Input image missing: {args.input}"}, 1)
    if not model.is_file() or model.stat().st_size < 1024:
        emit({"status": "error", "message": f"Matting model missing: {args.model}"}, 2)

    try:
        image = Image.open(source).convert("RGB")
        width, height = image.size
        size = max(32, args.size)
        resized = image.resize((size, size), Image.Resampling.BILINEAR)

        array = np.asarray(resized, dtype=np.float32) / 255.0
        array = (array.transpose(2, 0, 1) - MEAN) / STD
        tensor = np.ascontiguousarray(array[None, ...], dtype=np.float32)

        options = ort.SessionOptions()
        options.graph_optimization_level = ort.GraphOptimizationLevel.ORT_ENABLE_ALL
        options.intra_op_num_threads = 4
        session = ort.InferenceSession(str(model), options, providers=["CPUExecutionProvider"])
        # 静态形状权重：维度对不上就重跑一次，别让整条链路因为 320≠其它值而崩。
        expected = session.get_inputs()[0].shape
        if isinstance(expected[-1], int) and expected[-1] != size:
            size = int(expected[-1])
            resized = image.resize((size, size), Image.Resampling.BILINEAR)
            array = np.asarray(resized, dtype=np.float32) / 255.0
            array = (array.transpose(2, 0, 1) - MEAN) / STD
            tensor = np.ascontiguousarray(array[None, ...], dtype=np.float32)

        started = time.perf_counter()
        outputs = session.run(None, {MODEL_INPUT_NAME: tensor})
        elapsed_ms = round((time.perf_counter() - started) * 1000, 1)

        alpha = np.array(outputs[0])[0, 0]
        minimum, maximum = float(alpha.min()), float(alpha.max())
        alpha = (alpha - minimum) / (maximum - minimum + 1e-8)
        mask = Image.fromarray((np.clip(alpha, 0.0, 1.0) * 255.0).astype(np.uint8))
        if mask.size != (width, height):
            mask = mask.resize((width, height), Image.Resampling.BILINEAR)

        destination = Path(args.output)
        destination.parent.mkdir(parents=True, exist_ok=True)
        mask.save(destination, format="PNG")

        emit({
            "status": "ok",
            "model": model.name,
            "width": width,
            "height": height,
            "inference_size": size,
            "elapsed_ms": elapsed_ms,
            "foreground_ratio": round(float((np.asarray(mask) > 127).mean()), 4),
            "output": str(destination),
        })
    except Exception as error:
        emit({"status": "error", "message": f"Matting failed: {error}"}, 3)


if __name__ == "__main__":
    main()
