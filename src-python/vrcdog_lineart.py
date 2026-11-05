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


def directional_nms(x: np.ndarray) -> np.ndarray:
    """
    沿概率图自身梯度方向做局部极大值抑制（line-NMS 的 numpy 版）。

    旧实现是「4 个轴向里任一方向是峰值就保留」，那是 OR 语义：一条 3px 宽的粗脊
    在水平方向上三个像素全都 `>=` 左右邻居，于是整条脊都被留下 —— 实测
    PiDiNet base 在 0.12 阈值下墨迹占比 12%，画出来就是毛边和双线。
    这里按梯度把每个像素**唯一**分到 4 个法向之一，只跟该方向的两个邻居比，
    并要求一侧严格大于（打破平台区），粗脊就被压成 1px 中心线。
    """
    h, w = x.shape
    interior = np.zeros((h, w), dtype=bool)
    interior[1:-1, 1:-1] = True

    gx = np.zeros_like(x)
    gy = np.zeros_like(x)
    gx[:, 1:-1] = x[:, 2:] - x[:, :-2]
    gy[1:-1, :] = x[2:, :] - x[:-2, :]
    abs_x = np.abs(gx)
    abs_y = np.abs(gy)

    # 边缘的梯度方向就是它的法向；法向分成 4 个互斥扇区。
    sector_h = interior & (abs_x >= abs_y * 2.4142)          # 法向≈水平
    sector_v = interior & (abs_y >= abs_x * 2.4142)          # 法向≈垂直
    sector_d1 = interior & ~(sector_h | sector_v) & (abs_x >= abs_y)   # 法向≈45°（左上-右下）
    sector_d2 = interior & ~(sector_h | sector_v | sector_d1)          # 法向≈135°

    out = np.zeros((h, w), dtype=bool)

    left = np.zeros_like(x); left[:, 1:-1] = x[:, :-2]
    right = np.zeros_like(x); right[:, 1:-1] = x[:, 2:]
    out |= sector_h & (x > left) & (x >= right)

    up = np.zeros_like(x); up[1:-1, :] = x[:-2, :]
    down = np.zeros_like(x); down[1:-1, :] = x[2:, :]
    out |= sector_v & (x > up) & (x >= down)

    a = np.zeros_like(x); a[1:-1, 1:-1] = x[:-2, :-2]
    b = np.zeros_like(x); b[1:-1, 1:-1] = x[2:, 2:]
    out |= sector_d1 & (x > a) & (x >= b)

    c = np.zeros_like(x); c[1:-1, 1:-1] = x[2:, :-2]
    d = np.zeros_like(x); d[1:-1, 1:-1] = x[:-2, 2:]
    out |= sector_d2 & (x > c) & (x >= d)

    return out


def hysteresis_connect(peaks: np.ndarray, x: np.ndarray, low: float, high: float) -> np.ndarray:
    """
    双阈值滞后连接：>high 的强边缘作为种子，只保留能从种子可达的 >low 弱边缘。

    单一硬阈值的问题是二选一：调高丢细节（发丝、衣纹断成截），调低把屏幕摩尔纹
    和阴影噪点全收进来。滞后连接两头都要：弱边缘只有连在真线上才保留。
    """
    strong = peaks & (x >= high)
    weak = peaks & (x >= low) & ~strong
    result = strong.copy()
    # 最多迭代 512 次（约等于图的对角线长度），提前收敛就退出。
    for _ in range(512):
        grown = np.zeros_like(result)
        grown[1:, :] |= result[:-1, :]
        grown[:-1, :] |= result[1:, :]
        grown[:, 1:] |= result[:, :-1]
        grown[:, :-1] |= result[:, 1:]
        # 45° 也允许连通，否则斜线会在滞后阶段被截断。
        grown[1:, 1:] |= result[:-1, :-1]
        grown[:-1, :-1] |= result[1:, 1:]
        grown[1:, :-1] |= result[:-1, 1:]
        grown[:-1, 1:] |= result[1:, :-1]
        new = grown & weak & ~result
        if not new.any():
            break
        result |= new
    return result


def close_gaps(mask: np.ndarray, radius: int = 2) -> np.ndarray:
    """
    形态学闭运算桥接断点：先膨胀 radius 步再腐蚀 radius 步（4 邻域）。

    方向性 NMS + 滞后连接会把弱边缘全部丢掉，线稿因此干净但碎：实测 dog.jpg
    出 396 段，而旧管线只 227 段。VRChat 每次重新落笔都要等 min_button_down
    （默认 600ms）防双击橡皮，段数就是绘制时长的硬下限，所以必须把 2~4px 的
    小缺口接起来。闭运算只在缺口处留下连接丝，原有线宽基本不变。
    """
    if radius <= 0:
        return mask
    result = mask.astype(bool).copy()
    for _ in range(radius):
        grown = result.copy()
        grown[1:, :] |= result[:-1, :]
        grown[:-1, :] |= result[1:, :]
        grown[:, 1:] |= result[:, :-1]
        grown[:, :-1] |= result[:, 1:]
        result = grown
    for _ in range(radius):
        shrunk = result.copy()
        shrunk[1:, :] &= result[:-1, :]
        shrunk[:-1, :] &= result[1:, :]
        shrunk[:, 1:] &= result[:, :-1]
        shrunk[:, :-1] &= result[:, 1:]
        # 边界像素会被腐蚀直接抹掉，这里把外圈还原，避免贴边的轮廓线被啃短。
        shrunk[0, :] = result[0, :]
        shrunk[-1, :] = result[-1, :]
        shrunk[:, 0] = result[:, 0]
        shrunk[:, -1] = result[:, -1]
        result = shrunk
    return result.astype(np.uint8)


def nms_numpy(edge_float: np.ndarray, threshold: float = 0.12, blur_radius: float = 0.5) -> np.ndarray:
    """方向性 NMS + 双阈值滞后连接 + 闭运算桥接，输出 1px 干净且连续的线稿。"""
    h, w = edge_float.shape
    if h < 3 or w < 3:
        return (edge_float > threshold).astype(np.uint8) * 255

    if blur_radius > 0.05:
        pil_tmp = Image.fromarray((np.clip(edge_float, 0.0, 1.0) * 255.0).astype(np.uint8))
        pil_tmp = pil_tmp.filter(ImageFilter.GaussianBlur(radius=blur_radius))
        x = np.asarray(pil_tmp, dtype=np.float32) / 255.0
    else:
        x = edge_float.astype(np.float32)

    peaks = directional_nms(x)
    low = float(np.clip(threshold, 0.01, 0.6))
    high = float(np.clip(low * 2.2, low + 0.02, 0.85))
    kept = hysteresis_connect(peaks, x, low, high)
    # 半径 2 能接住 ~4px 的缺口；半径 3 开始把相邻的细线粘在一起（样张实测）。
    kept = close_gaps(kept, radius=2)
    return kept.astype(np.uint8) * 255


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
