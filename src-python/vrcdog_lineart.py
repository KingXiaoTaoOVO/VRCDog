#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
VRCDog AI Line Art Extractor (PiDiNet ONNX Engine)
Extracts crisp, clean, noise-free contour line art from any image locally.
"""

import os
import sys
import json
import argparse
from pathlib import Path
import numpy as np

# Suppress ONNX Runtime verbosity
os.environ["ORT_LOG_LEVEL"] = "3"

try:
    import onnxruntime as ort
    from PIL import Image, ImageFilter
except ImportError as e:
    print(json.dumps({"status": "error", "message": f"Missing required Python dependencies: {e}"}))
    sys.exit(1)


MEAN = np.array([0.485, 0.456, 0.406], dtype=np.float32)[:, None, None]
STD = np.array([0.229, 0.224, 0.225], dtype=np.float32)[:, None, None]


def locate_model(model_name: str, custom_dir: str = None) -> Path:
    """Finds the ONNX model file on disk."""
    base_name = "table5_pidinet_tiny.onnx"
    if any(k in model_name.lower() for k in ["base", "anime2sketch", "pidinet_table5"]):
        base_name = "pidinet_table5.onnx"

    candidates = []
    if custom_dir:
        candidates.append(Path(custom_dir) / base_name)

    here = Path(__file__).resolve().parent
    candidates.extend([
        here.parent / "src-tauri" / "resources" / "models" / base_name,
        here.parent / "resources" / "models" / base_name,
        here / "resources" / "models" / base_name,
        here / "models" / base_name,
        Path.cwd() / "src-tauri" / "resources" / "models" / base_name,
        Path.cwd() / "resources" / "models" / base_name,
    ])

    for c in candidates:
        if c.is_file() and c.stat().st_size > 1024:
            return c

    # Fallback to tiny if base was requested but not found
    if base_name != "table5_pidinet_tiny.onnx":
        for c in candidates:
            alt = c.parent / "table5_pidinet_tiny.onnx"
            if alt.is_file() and alt.stat().st_size > 1024:
                return alt

    raise FileNotFoundError(f"Model file '{base_name}' not found in candidate paths.")


def nms_numpy(edge_float: np.ndarray, threshold: float = 0.12, blur_radius: float = 0.5) -> np.ndarray:
    """
    4-directional Non-Maximum Suppression (NMS) to thin edge gradients into
    single-pixel crisp lines while suppressing noise.
    """
    h, w = edge_float.shape
    if h < 3 or w < 3:
        return (edge_float > threshold).astype(np.uint8) * 255

    # Optional Gaussian smoothing before local peak picking
    if blur_radius > 0.05:
        pil_tmp = Image.fromarray((np.clip(edge_float, 0.0, 1.0) * 255.0).astype(np.uint8))
        pil_tmp = pil_tmp.filter(ImageFilter.GaussianBlur(radius=blur_radius))
        x = np.asarray(pil_tmp, dtype=np.float32) / 255.0
    else:
        x = edge_float

    c = x[1:-1, 1:-1]
    # Direction 1: horizontal (left, right)
    m1 = (c >= x[1:-1, :-2]) & (c >= x[1:-1, 2:])
    # Direction 2: vertical (up, down)
    m2 = (c >= x[:-2, 1:-1]) & (c >= x[2:, 1:-1])
    # Direction 3: diagonal top-left to bottom-right
    m3 = (c >= x[:-2, :-2]) & (c >= x[2:, 2:])
    # Direction 4: diagonal top-right to bottom-left
    m4 = (c >= x[:-2, 2:]) & (c >= x[2:, :-2])

    is_peak = m1 | m2 | m3 | m4
    valid = is_peak & (c > threshold)

    out = np.zeros_like(x, dtype=np.uint8)
    out[1:-1, 1:-1][valid] = 255
    return out


def main():
    parser = argparse.ArgumentParser(description="VRCDog AI Line Art Inference")
    parser.add_argument("--input", "-i", required=True, help="Path to input image")
    parser.add_argument("--output", "-o", required=True, help="Path to save output binary mask PNG")
    parser.add_argument("--model", "-m", default="image-to-line", help="Model name (pidinet_tiny / pidinet_base)")
    parser.add_argument("--max-dim", type=int, default=512, help="Max image dimension for inference")
    parser.add_argument("--contrast", type=float, default=1.0, help="Contrast adjustment factor")
    parser.add_argument("--threshold", type=float, default=0.12, help="NMS sensitivity threshold (0.01 - 0.5)")
    parser.add_argument("--models-dir", default=None, help="Custom directory containing models")

    args = parser.parse_args()

    input_path = Path(args.input)
    if not input_path.is_file():
        print(json.dumps({"status": "error", "message": f"Input image does not exist: {args.input}"}))
        sys.exit(1)

    try:
        model_path = locate_model(args.model, args.models_dir)
    except Exception as e:
        print(json.dumps({"status": "error", "message": str(e)}))
        sys.exit(2)

    try:
        img = Image.open(input_path).convert("RGB")
        orig_w, orig_h = img.size

        # PiDiNet stride constraints require dimensions divisible by 32
        max_dim = max(128, min(args.max_dim, 1024))
        scale = min(max_dim / max(orig_w, orig_h), 1.0)
        net_w = max(32, int(round((orig_w * scale) / 32.0)) * 32)
        net_h = max(32, int(round((orig_h * scale) / 32.0)) * 32)

        img_resized = img.resize((net_w, net_h), Image.Resampling.BILINEAR)

        # Contrast adjustment if specified
        arr = np.asarray(img_resized, dtype=np.float32) / 255.0
        if abs(args.contrast - 1.0) > 0.05:
            arr = np.clip((arr - 0.5) * args.contrast + 0.5, 0.0, 1.0)

        # ImageNet normalization
        arr = arr.transpose(2, 0, 1)  # CHW
        arr = (arr - MEAN) / STD
        inp = arr[None, ...]  # BCHW

        # Create ONNX session
        avail = ort.get_available_providers()
        providers = []
        if "CUDAExecutionProvider" in avail:
            providers.append("CUDAExecutionProvider")
        if "DirectMLExecutionProvider" in avail:
            providers.append("DirectMLExecutionProvider")
        providers.append("CPUExecutionProvider")

        sess_options = ort.SessionOptions()
        sess_options.graph_optimization_level = ort.GraphOptimizationLevel.ORT_ENABLE_ALL
        sess_options.intra_op_num_threads = 4
        session = ort.InferenceSession(str(model_path), sess_options, providers=providers)

        outputs = session.run(None, {"image": inp})
        fused = np.array(outputs[-1])[0, 0]  # shape: (net_h, net_w)

        # Non-Maximum Suppression to get single-pixel crisp lines
        effective_threshold = float(np.clip(args.threshold, 0.03, 0.40))
        thin_edges = nms_numpy(fused, threshold=effective_threshold, blur_radius=0.5)

        # Resize mask back to original or target dimensions
        mask_pil = Image.fromarray(thin_edges).resize((orig_w, orig_h), Image.Resampling.NEAREST)

        out_path = Path(args.output)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        mask_pil.save(out_path, format="PNG")

        ink_count = int(np.sum(np.array(mask_pil) > 0))

        print(json.dumps({
            "status": "ok",
            "model": model_path.name,
            "width": orig_w,
            "height": orig_h,
            "ink_pixels": ink_count,
            "output": str(out_path)
        }))
        sys.exit(0)

    except Exception as e:
        print(json.dumps({"status": "error", "message": f"Inference failed: {e}"}))
        sys.exit(3)


if __name__ == "__main__":
    main()
