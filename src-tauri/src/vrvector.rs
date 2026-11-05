//! 带墨迹证据门禁的矢量拟合（移植自 references/VRC-Draw/src/VectorPath.cpp）。
//!
//! 为什么不用现成的 RDP 折线：`simplify_points` 只保证「几何误差 ≤ epsilon」，
//! 它会把圆弧切成 1.6px 的直边（放大看就是多边形），也会把本可以是一条直线的
//! 骨架点保留成几十个抖动点。真正的流线需要的是
//! 「在容差内用最少的三次曲线段」，并且**每一步都要有原图墨迹背书**——
//! 拟合出来的曲线一旦离开墨迹就必须整段退回折线，宁可留锯齿也不画脱离图像的线。
//!
//! 三层门禁（与参考实现一致）：
//! 1. 双向 Hausdorff 距离 ≤ curve_tolerance；
//! 2. 候选折线逐段采样（密度 2×段长）必须全部落在 3×3 墨迹邻域内；
//! 3. 自交次数与曲率尖峰数不得高于原折线，且不得越出原折线包围盒 + 容差。

use crate::vrdrawing::DrawingPoint;

const GEOMETRY_EPSILON: f32 = 1e-5;

#[derive(Clone, Copy, Debug)]
pub struct VectorFitOptions {
    /// 判定「这段就是直线」的偏移上限（像素）。
    pub line_tolerance: f32,
    /// 单条三次贝塞尔可接受的最大误差，也是所有门禁的越界预算。
    pub curve_tolerance: f32,
    /// 展平（曲线→落笔点）时的平坦度容差，按执行尺度反算。
    pub flattening_tolerance: f32,
    /// 角点判定时两侧臂长（像素）。
    pub corner_arm_length: f32,
    /// 夹角余弦高于此值算「需要保护的角点」（-0.45 ≈ 转角 < 116.7°）。
    pub corner_cosine_threshold: f32,
}

impl Default for VectorFitOptions {
    fn default() -> Self {
        Self {
            line_tolerance: 0.45,
            curve_tolerance: 0.85,
            flattening_tolerance: 0.25,
            corner_arm_length: 4.0,
            corner_cosine_threshold: -0.45,
        }
    }
}

/// 执行尺度（1 图像像素 → 多少鼠标命令单位）反算展平容差。
///
/// 放大画布时同样的曲线需要更密的落点，缩小时可以更疏；
/// 参考实现取 `clamp(0.25 / scale, 0.08, 0.75)`，这里保持一致。
pub fn flattening_tolerance_for_scale(effective_scale: f32) -> f32 {
    if !effective_scale.is_finite() || effective_scale <= 0.0 {
        return 0.25;
    }
    (0.25 / effective_scale).clamp(0.08, 0.75)
}

#[derive(Clone, Copy)]
struct Cubic {
    p0: DrawingPoint,
    p1: DrawingPoint,
    p2: DrawingPoint,
    p3: DrawingPoint,
}

#[derive(Clone, Copy)]
enum Segment {
    Line { end: DrawingPoint },
    Cubic(Cubic),
}

fn sub(a: &DrawingPoint, b: &DrawingPoint) -> (f32, f32) { (a.x - b.x, a.y - b.y) }
fn add_mul(point: DrawingPoint, vector: (f32, f32), scale: f32) -> DrawingPoint {
    DrawingPoint { x: point.x + vector.0 * scale, y: point.y + vector.1 * scale }
}
fn dot(a: (f32, f32), b: (f32, f32)) -> f32 { a.0 * b.0 + a.1 * b.1 }
fn len(a: (f32, f32)) -> f32 { a.0.hypot(a.1) }
fn normalize(a: (f32, f32)) -> (f32, f32) {
    let l = len(a);
    if l > GEOMETRY_EPSILON { (a.0 / l, a.1 / l) } else { (0.0, 0.0) }
}

fn evaluate(curve: &Cubic, t: f32) -> DrawingPoint {
    let u = 1.0 - t;
    let b0 = u * u * u;
    let b1 = 3.0 * u * u * t;
    let b2 = 3.0 * u * t * t;
    let b3 = t * t * t;
    DrawingPoint {
        x: curve.p0.x * b0 + curve.p1.x * b1 + curve.p2.x * b2 + curve.p3.x * b3,
        y: curve.p0.y * b0 + curve.p1.y * b1 + curve.p2.y * b2 + curve.p3.y * b3,
    }
}

/// de Casteljau 二分。
fn split_curve(curve: &Cubic) -> (Cubic, Cubic) {
    let mid = |a: DrawingPoint, b: DrawingPoint| DrawingPoint { x: (a.x + b.x) * 0.5, y: (a.y + b.y) * 0.5 };
    let p01 = mid(curve.p0, curve.p1);
    let p12 = mid(curve.p1, curve.p2);
    let p23 = mid(curve.p2, curve.p3);
    let p012 = mid(p01, p12);
    let p123 = mid(p12, p23);
    let center = mid(p012, p123);
    (
        Cubic { p0: curve.p0, p1: p01, p2: p012, p3: center },
        Cubic { p0: center, p1: p123, p2: p23, p3: curve.p3 },
    )
}

fn point_segment_distance(point: &DrawingPoint, start: &DrawingPoint, end: &DrawingPoint) -> f32 {
    let vx = end.x - start.x;
    let vy = end.y - start.y;
    let wx = point.x - start.x;
    let wy = point.y - start.y;
    let denominator = vx * vx + vy * vy;
    if denominator <= GEOMETRY_EPSILON { return wx.hypot(wy); }
    let t = ((wx * vx + wy * vy) / denominator).clamp(0.0, 1.0);
    (wx - t * vx).hypot(wy - t * vy)
}

fn point_polyline_distance(point: &DrawingPoint, polyline: &[DrawingPoint]) -> f32 {
    if polyline.is_empty() { return f32::INFINITY; }
    if polyline.len() == 1 { return len(sub(point, &polyline[0])); }
    let mut distance = f32::INFINITY;
    for index in 1..polyline.len() {
        distance = distance.min(point_segment_distance(point, &polyline[index - 1], &polyline[index]));
    }
    distance
}

fn bidirectional_distance(left: &[DrawingPoint], right: &[DrawingPoint]) -> f32 {
    let mut distance = 0.0f32;
    for point in left { distance = distance.max(point_polyline_distance(point, right)); }
    for point in right { distance = distance.max(point_polyline_distance(point, left)); }
    distance
}

fn orientation(a: &DrawingPoint, b: &DrawingPoint, c: &DrawingPoint) -> i32 {
    let value = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    if value.abs() <= 1e-4 { 0 } else if value > 0.0 { 1 } else { -1 }
}

fn properly_intersects(a: &DrawingPoint, b: &DrawingPoint, c: &DrawingPoint, d: &DrawingPoint) -> bool {
    orientation(a, b, c) * orientation(a, b, d) < 0 && orientation(c, d, a) * orientation(c, d, b) < 0
}

fn self_intersection_count(points: &[DrawingPoint]) -> usize {
    if points.len() < 4 { return 0; }
    let closed = points[0].x == points[points.len() - 1].x && points[0].y == points[points.len() - 1].y;
    let mut count = 0usize;
    for left in 1..points.len() {
        for right in (left + 2)..points.len() {
            if closed && left == 1 && right + 1 == points.len() { continue; }
            if properly_intersects(&points[left - 1], &points[left], &points[right - 1], &points[right]) {
                count += 1;
            }
        }
    }
    count
}

fn curvature_spike_count(points: &[DrawingPoint]) -> usize {
    let mut count = 0usize;
    for index in 2..points.len() {
        let incoming = sub(&points[index - 1], &points[index - 2]);
        let outgoing = sub(&points[index], &points[index - 1]);
        let denominator = len(incoming) * len(outgoing);
        if denominator > GEOMETRY_EPSILON && dot(incoming, outgoing) / denominator < 0.35 {
            count += 1;
        }
    }
    count
}

fn point_supported_by_ink(point: &DrawingPoint, ink: &[u8], width: usize, height: usize) -> bool {
    if width == 0 || height == 0 || ink.len() != width * height { return false; }
    let center_x = point.x.round() as i64;
    let center_y = point.y.round() as i64;
    for offset_y in -1..=1i64 {
        for offset_x in -1..=1i64 {
            let x = center_x + offset_x;
            let y = center_y + offset_y;
            if x < 0 || y < 0 || x >= width as i64 || y >= height as i64 { continue; }
            if ink[(y as usize) * width + x as usize] != 0 { return true; }
        }
    }
    false
}

fn polyline_supported_by_ink(points: &[DrawingPoint], ink: &[u8], width: usize, height: usize) -> bool {
    if points.len() < 2 { return false; }
    for index in 1..points.len() {
        let start = &points[index - 1];
        let end = &points[index];
        let segment_length = len(sub(end, start));
        let steps = (segment_length * 2.0).ceil().max(1.0) as usize;
        for step in 0..=steps {
            let ratio = step as f32 / steps as f32;
            let sample = DrawingPoint {
                x: start.x + (end.x - start.x) * ratio,
                y: start.y + (end.y - start.y) * ratio,
            };
            if !point_supported_by_ink(&sample, ink, width, height) { return false; }
        }
    }
    true
}

fn flatten_curve(curve: &Cubic, tolerance: f32, depth: u32, output: &mut Vec<DrawingPoint>) {
    let flatness = point_segment_distance(&curve.p1, &curve.p0, &curve.p3)
        .max(point_segment_distance(&curve.p2, &curve.p0, &curve.p3));
    if flatness <= tolerance || depth >= 16 {
        let last = output.last();
        if last.map(|p| p.x != curve.p3.x || p.y != curve.p3.y).unwrap_or(true) {
            output.push(curve.p3);
        }
        return;
    }
    let (left, right) = split_curve(curve);
    flatten_curve(&left, tolerance, depth + 1, output);
    flatten_curve(&right, tolerance, depth + 1, output);
}

/// 以两端真实切线为方向、`alpha0/alpha1` 为未知量的 2×2 最小二乘。
/// 解越出 `[0.02·chord, 1.25·chord]` 就退回 `chord/3`；反向柄直接拒绝。
/// 返回 `None` 表示最大误差超容差，`worst` 给出误差最大的点供二分。
fn fit_single_cubic(points: &[DrawingPoint], tolerance: f32, worst: &mut usize) -> Option<Cubic> {
    if points.len() < 4 { return None; }
    let p0 = points[0];
    let p3 = points[points.len() - 1];
    let chord = len(sub(&p3, &p0));
    if chord <= GEOMETRY_EPSILON { return None; }
    let start_tangent = normalize(sub(&points[1], &p0));
    let end_tangent = normalize(sub(&points[points.len() - 2], &p3));
    if len(start_tangent) < 0.5 || len(end_tangent) < 0.5 { return None; }

    // 参数用累计弧长归一化（不是索引等分），长短线段的权重才正确。
    let mut parameters = vec![0.0f32; points.len()];
    let mut total_length = 0.0f32;
    for index in 1..points.len() {
        total_length += len(sub(&points[index], &points[index - 1]));
        parameters[index] = total_length;
    }
    if total_length <= GEOMETRY_EPSILON { return None; }
    for parameter in &mut parameters { *parameter /= total_length; }

    let mut c00 = 0.0f32; let mut c01 = 0.0f32; let mut c11 = 0.0f32;
    let mut x0 = 0.0f32; let mut x1 = 0.0f32;
    for (index, point) in points.iter().enumerate() {
        let t = parameters[index];
        let u = 1.0 - t;
        let b1 = 3.0 * u * u * t;
        let b2 = 3.0 * u * t * t;
        let base = DrawingPoint {
            x: p0.x * (u * u * u + b1) + p3.x * (b2 + t * t * t),
            y: p0.y * (u * u * u + b1) + p3.y * (b2 + t * t * t),
        };
        let residual = sub(point, &base);
        let a0 = (start_tangent.0 * b1, start_tangent.1 * b1);
        let a1 = (end_tangent.0 * b2, end_tangent.1 * b2);
        c00 += dot(a0, a0);
        c01 += dot(a0, a1);
        c11 += dot(a1, a1);
        x0 += dot(a0, residual);
        x1 += dot(a1, residual);
    }
    let determinant = c00 * c11 - c01 * c01;
    let mut alpha0 = chord / 3.0;
    let mut alpha1 = chord / 3.0;
    if determinant.abs() > 1e-6 {
        alpha0 = (x0 * c11 - x1 * c01) / determinant;
        alpha1 = (c00 * x1 - c01 * x0) / determinant;
    }
    if alpha0 < chord * 0.02 || alpha1 < chord * 0.02 || alpha0 > chord * 1.25 || alpha1 > chord * 1.25 {
        alpha0 = chord / 3.0;
        alpha1 = chord / 3.0;
    }
    let curve = Cubic {
        p0,
        p1: add_mul(p0, start_tangent, alpha0),
        p2: add_mul(p3, end_tangent, alpha1),
        p3,
    };
    if dot(sub(&curve.p1, &p0), sub(&p3, &p0)) < 0.0 || dot(sub(&curve.p2, &p3), sub(&p0, &p3)) < 0.0 {
        return None;
    }

    let mut maximum_error = 0.0f32;
    *worst = points.len() / 2;
    for index in 1..points.len() - 1 {
        let error = len(sub(&evaluate(&curve, parameters[index]), &points[index]));
        if error > maximum_error {
            maximum_error = error;
            *worst = index;
        }
    }
    if maximum_error <= tolerance { Some(curve) } else { None }
}

/// 先试直线，再试单条三次贝塞尔，都不行就在误差最大点二分递归。
fn fit_recursive(points: &[DrawingPoint], options: &VectorFitOptions, output: &mut Vec<Segment>) {
    if points.len() < 2 { return; }
    let mut maximum_line_distance = 0.0f32;
    let mut line_split = points.len() / 2;
    for index in 1..points.len() - 1 {
        let distance = point_segment_distance(&points[index], &points[0], &points[points.len() - 1]);
        if distance > maximum_line_distance {
            maximum_line_distance = distance;
            line_split = index;
        }
    }
    if maximum_line_distance <= options.line_tolerance {
        output.push(Segment::Line { end: points[points.len() - 1] });
        return;
    }

    let mut worst = line_split;
    if let Some(curve) = fit_single_cubic(points, options.curve_tolerance, &mut worst) {
        output.push(Segment::Cubic(curve));
        return;
    }
    if points.len() <= 3 {
        for index in 1..points.len() {
            output.push(Segment::Line { end: points[index] });
        }
        return;
    }
    worst = worst.clamp(1, points.len() - 2);
    fit_recursive(&points[..=worst], options, output);
    fit_recursive(&points[worst..], options, output);
}

fn flatten_segments(start: DrawingPoint, segments: &[Segment], tolerance: f32) -> Vec<DrawingPoint> {
    let mut result = vec![start];
    let mut current = start;
    for segment in segments {
        match segment {
            Segment::Cubic(curve) => {
                let mut fixed = *curve;
                fixed.p0 = current;
                flatten_curve(&fixed, tolerance.max(0.01), 0, &mut result);
                current = fixed.p3;
            }
            Segment::Line { end } => {
                let last = result.last();
                if last.map(|p| p.x != end.x || p.y != end.y).unwrap_or(true) {
                    result.push(*end);
                }
                current = *end;
            }
        }
    }
    result
}

/// 拟合结果必须处处不比原折线差，否则整段退回折线。
fn candidate_is_safe(
    fallback: &[DrawingPoint],
    segments: &[Segment],
    ink: &[u8],
    width: usize,
    height: usize,
    options: &VectorFitOptions,
) -> bool {
    if fallback.len() < 2 || segments.is_empty() { return false; }
    let candidate = flatten_segments(fallback[0], segments, options.flattening_tolerance);
    if candidate.len() < 2 { return false; }
    if candidate[0].x != fallback[0].x || candidate[0].y != fallback[0].y { return false; }
    let tail_ok = candidate[candidate.len() - 1].x == fallback[fallback.len() - 1].x
        && candidate[candidate.len() - 1].y == fallback[fallback.len() - 1].y;
    if !tail_ok { return false; }
    if bidirectional_distance(&candidate, fallback) > options.curve_tolerance { return false; }
    if !polyline_supported_by_ink(&candidate, ink, width, height) { return false; }
    if self_intersection_count(&candidate) > self_intersection_count(fallback) { return false; }
    if curvature_spike_count(&candidate) > curvature_spike_count(fallback) { return false; }
    let mut minimum_x = fallback[0].x; let mut maximum_x = fallback[0].x;
    let mut minimum_y = fallback[0].y; let mut maximum_y = fallback[0].y;
    for point in fallback {
        minimum_x = minimum_x.min(point.x);
        maximum_x = maximum_x.max(point.x);
        minimum_y = minimum_y.min(point.y);
        maximum_y = maximum_y.max(point.y);
    }
    candidate.iter().all(|point| {
        point.x >= minimum_x - options.curve_tolerance
            && point.x <= maximum_x + options.curve_tolerance
            && point.y >= minimum_y - options.curve_tolerance
            && point.y <= maximum_y + options.curve_tolerance
    })
}

/// 硬锚点：两侧各累计 `corner_arm_length` 像素的臂，夹角余弦高于阈值就是需要保护的角。
/// 拟合永远不许跨越角点，否则尖角会被贝塞尔圆化。
fn protected_corners(points: &[DrawingPoint], options: &VectorFitOptions) -> Vec<usize> {
    let mut candidates: Vec<(usize, f32)> = Vec::new();
    for index in 1..points.len().saturating_sub(1) {
        let mut left = index;
        let mut left_length = 0.0f32;
        while left > 0 && left_length < options.corner_arm_length {
            left_length += len(sub(&points[left], &points[left - 1]));
            left -= 1;
        }
        let mut right = index;
        let mut right_length = 0.0f32;
        while right + 1 < points.len() && right_length < options.corner_arm_length {
            right_length += len(sub(&points[right + 1], &points[right]));
            right += 1;
        }
        if left_length < 1.5 || right_length < 1.5 { continue; }
        let left_arm = normalize(sub(&points[left], &points[index]));
        let right_arm = normalize(sub(&points[right], &points[index]));
        let cosine = dot(left_arm, right_arm);
        if cosine > options.corner_cosine_threshold {
            candidates.push((index, cosine));
        }
    }
    candidates.sort_by_key(|(index, _)| *index);
    let mut result: Vec<usize> = Vec::new();
    for (index, cosine) in candidates.iter() {
        if let Some(last) = result.last_mut() {
            if *index - *last <= 2 {
                let previous = candidates.iter().find(|(candidate, _)| candidate == last);
                if let Some((_, previous_cosine)) = previous {
                    if cosine > previous_cosine { *last = *index; }
                }
                continue;
            }
        }
        result.push(*index);
    }
    result
}

fn lexicographically_less(a: &DrawingPoint, b: &DrawingPoint) -> bool {
    (a.x, a.y) < (b.x, b.y)
}

/// 把一条栅格骨架折线拟合成「最少三次曲线段」并展平回放点。
///
/// 返回 `None` = 任何一层门禁没过（越出墨迹、双向偏差超限、自交/尖峰变多、
/// 段数没减少）。调用方必须自己准备回退方案，**不要**在这里偷偷退回输入：
/// 直接把稠密骨架点当结果吐出去，落点数会翻好几倍，绘制时间跟着炸。
pub fn fit_stroke(
    points: &[DrawingPoint],
    ink: &[u8],
    width: usize,
    height: usize,
    options: &VectorFitOptions,
) -> Option<Vec<DrawingPoint>> {
    if points.len() < 3 { return None; }
    let reverse = lexicographically_less(&points[points.len() - 1], &points[0]);
    let mut canonical = points.to_vec();
    if reverse { canonical.reverse(); }

    // 只在「原始折线段边界 + 受保护角点」处切分，保证角点两侧分别拟合。
    let mut boundaries = vec![0usize];
    boundaries.extend(protected_corners(&canonical, options));
    boundaries.push(canonical.len() - 1);
    boundaries.sort_unstable();
    boundaries.dedup();

    let mut segments: Vec<Segment> = Vec::new();
    for part in 1..boundaries.len() {
        let begin = boundaries[part - 1];
        let end = boundaries[part];
        if end <= begin { continue; }
        let part_points = &canonical[begin..=end];
        if part_points.len() < 3 {
            for point in part_points.iter().skip(1) {
                segments.push(Segment::Line { end: *point });
            }
            continue;
        }
        fit_recursive(part_points, options, &mut segments);
    }
    // 段数没有减少就说明拟合没带来任何收益（参考实现同判据）。
    if segments.len() >= canonical.len() - 1 { return None; }
    if !candidate_is_safe(&canonical, &segments, ink, width, height, options) { return None; }

    let flattened = flatten_segments(canonical[0], &segments, options.flattening_tolerance);
    if flattened.len() < 2 { return None; }
    if !polyline_supported_by_ink(&flattened, ink, width, height) { return None; }
    if reverse {
        // 反向只反转结果，绝不重新拟合：重拟合会得到另一组曲线。
        let mut reversed = flattened;
        reversed.reverse();
        Some(reversed)
    } else {
        Some(flattened)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank(width: usize, height: usize) -> Vec<u8> { vec![0u8; width * height] }

    /// 把折线按整数像素盖进墨迹图（`polyline_supported_by_ink` 有 3×3 容差，够用）。
    fn stamp(mask: &mut Vec<u8>, width: usize, height: usize, points: &[DrawingPoint]) {
        for index in 1..points.len() {
            let a = points[index - 1];
            let b = points[index];
            let steps = ((b.x - a.x).abs().max((b.y - a.y).abs()).ceil() as usize).max(1);
            for step in 0..=steps {
                let ratio = step as f32 / steps as f32;
                let x = (a.x + (b.x - a.x) * ratio).round() as i64;
                let y = (a.y + (b.y - a.y) * ratio).round() as i64;
                if x < 0 || y < 0 || x >= width as i64 || y >= height as i64 { continue; }
                mask[(y as usize) * width + x as usize] = 1;
            }
        }
    }

    #[test]
    fn a_straight_line_with_tremor_collapses_to_its_endpoints() {
        let points: Vec<DrawingPoint> = (0..=40)
            .map(|index| DrawingPoint { x: index as f32, y: 2.0 + if index % 2 == 0 { -0.2 } else { 0.2 } })
            .collect();
        let mut ink = blank(48, 8);
        stamp(&mut ink, 48, 8, &points.iter().map(|p| DrawingPoint { x: p.x, y: 2.0 }).collect::<Vec<_>>());

        let fitted = fit_stroke(&points, &ink, 48, 8, &VectorFitOptions::default()).expect("直线必须拟合成功");
        assert_eq!(fitted.len(), 2, "抖动直线必须被认成一条直线，实际 {fitted:?}");
        assert_eq!(fitted[0].x, 0.0);
        assert_eq!(fitted[1].x, 40.0);
    }

    #[test]
    fn an_arc_is_fitted_instead_of_being_chorded() {
        let points: Vec<DrawingPoint> = (0..=80)
            .map(|degree| {
                let radians = (degree as f32 - 40.0).to_radians() * 2.0;
                DrawingPoint { x: 40.0 + 30.0 * radians.cos(), y: 40.0 + 30.0 * radians.sin() }
            })
            .collect();
        let mut ink = blank(80, 80);
        stamp(&mut ink, 80, 80, &points);

        let fitted = fit_stroke(&points, &ink, 80, 80, &VectorFitOptions::default()).expect("圆弧必须拟合成功");
        assert!(fitted.len() < points.len(), "圆弧应该被拟合成少量曲线段，得到 {} 点", fitted.len());
        assert!(fitted.len() >= 4, "展平后仍要有足够的落点密度，只有 {} 点", fitted.len());
        for point in &fitted {
            let radius = ((point.x - 40.0).powi(2) + (point.y - 40.0).powi(2)).sqrt();
            assert!((radius - 30.0).abs() <= 1.2, "拟合点脱离圆弧：半径 {radius}");
        }
    }

    #[test]
    fn fitting_is_rejected_when_the_curve_would_leave_the_ink() {
        let points: Vec<DrawingPoint> = (0..=80)
            .map(|degree| {
                let radians = (degree as f32 - 40.0).to_radians() * 2.0;
                DrawingPoint { x: 40.0 + 30.0 * radians.cos(), y: 40.0 + 30.0 * radians.sin() }
            })
            .collect();
        // 完全没有墨迹：任何越出原图的曲线都必须被拒绝，由调用方退回简化折线。
        let ink = blank(80, 80);
        assert!(
            fit_stroke(&points, &ink, 80, 80, &VectorFitOptions::default()).is_none(),
            "墨迹不支持时绝不能返回拟合结果"
        );
    }

    #[test]
    fn a_sharp_corner_survives_the_fit() {
        let mut points = Vec::new();
        for index in 0..=20 { points.push(DrawingPoint { x: 10.0, y: 10.0 + index as f32 * 2.0 }); }
        for index in 1..=20 { points.push(DrawingPoint { x: 10.0 + index as f32 * 2.0, y: 50.0 }); }
        let mut ink = blank(60, 60);
        stamp(&mut ink, 60, 60, &points);

        let fitted = fit_stroke(&points, &ink, 60, 60, &VectorFitOptions::default()).expect("直角折线至少要保住两条边");
        let corner_kept = fitted.iter().any(|point| (point.x - 10.0).abs() <= 0.6 && (point.y - 50.0).abs() <= 0.6);
        assert!(corner_kept, "直角被贝塞尔圆化了：{fitted:?}");
    }

    #[test]
    fn denser_flattening_is_used_when_the_canvas_is_enlarged() {
        assert!(flattening_tolerance_for_scale(2.0) < flattening_tolerance_for_scale(0.5));
        assert_eq!(flattening_tolerance_for_scale(0.0), 0.25);
        assert_eq!(flattening_tolerance_for_scale(100.0), 0.08);
    }
}
