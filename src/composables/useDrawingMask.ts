import { ref, watch, type Ref } from 'vue';

export type MaskTool = 'keep' | 'erase' | 'wand-erase' | 'wand-keep' | 'rect-keep' | 'rect-erase';

interface MaskSize { width: number; height: number }

/**
 * 绘画选区遮罩：白 = 要画，黑 = 不画。
 *
 * 为什么在浏览器里画而不是在 Rust 里：用户要的是「边看边涂边擦」的即时反馈，
 * 遮罩只有在预览坐标系里才画得准；Rust 侧只负责把它当成前景掩码用
 * （`vrdrawing_prepare` 的 `maskPng` 参数，非空时权威覆盖自动抠图）。
 */
export function useDrawingMask(sourceUrl: Ref<string>, plan: Ref<{ width: number; height: number } | null>) {
  const enabled = ref(false);
  const tool = ref<MaskTool>('erase');
  const brushSize = ref(24);
  const tolerance = ref(0.28);
  const version = ref(0);
  const size = ref<MaskSize>({ width: 0, height: 0 });

  let maskCanvas: HTMLCanvasElement | null = null;
  let maskCtx: CanvasRenderingContext2D | null = null;
  let sourceCanvas: HTMLCanvasElement | null = null;

  const targetSize = (): MaskSize => ({
    width: Math.max(1, plan.value?.width || 0),
    height: Math.max(1, plan.value?.height || 0),
  });

  const ensureMask = () => {
    const next = targetSize();
    if (!next.width || !next.height) return null;
    // 尺寸没变就沿用现有遮罩（用户已经涂过的内容不能丢）。
    if (maskCanvas && size.value.width === next.width && size.value.height === next.height) return maskCtx;
    maskCanvas = document.createElement('canvas');
    maskCanvas.width = next.width;
    maskCanvas.height = next.height;
    maskCtx = maskCanvas.getContext('2d', { willReadFrequently: true });
    if (!maskCtx) return null;
    // 换图 / 换尺寸 → 新遮罩默认「全画」，旧选区不跨图沿用。
    maskCtx.fillStyle = '#ffffff';
    maskCtx.fillRect(0, 0, next.width, next.height);
    size.value = next;
    version.value += 1;
    return maskCtx;
  };

  const resetMask = () => {
    const context = ensureMask();
    if (!context || !maskCanvas) return;
    context.fillStyle = '#ffffff';
    context.fillRect(0, 0, maskCanvas.width, maskCanvas.height);
    version.value += 1;
  };

  const invertMask = () => {
    const context = ensureMask();
    if (!context || !maskCanvas) return;
    const image = context.getImageData(0, 0, maskCanvas.width, maskCanvas.height);
    for (let index = 0; index < image.data.length; index += 4) {
      const value = image.data[index] > 127 ? 0 : 255;
      image.data[index] = value;
      image.data[index + 1] = value;
      image.data[index + 2] = value;
    }
    context.putImageData(image, 0, 0);
    version.value += 1;
  };

  /** 载入源图到 plan 尺寸的离屏画布，供魔棒取样。 */
  const loadSource = async (): Promise<boolean> => {
    const next = targetSize();
    if (sourceCanvas && sourceCanvas.width === next.width && sourceCanvas.height === next.height) return true;
    if (!sourceUrl.value) return false;
    const image = new Image();
    image.crossOrigin = 'anonymous';
    image.src = sourceUrl.value;
    await new Promise<void>((resolve, reject) => {
      image.onload = () => resolve();
      image.onerror = () => reject(new Error('source image failed'));
    });
    sourceCanvas = document.createElement('canvas');
    sourceCanvas.width = next.width;
    sourceCanvas.height = next.height;
    const context = sourceCanvas.getContext('2d', { willReadFrequently: true });
    if (!context) return false;
    context.drawImage(image, 0, 0, next.width, next.height);
    return true;
  };

  const paintCircle = (x: number, y: number, keep: boolean) => {
    const context = ensureMask();
    if (!context) return;
    context.fillStyle = keep ? '#ffffff' : '#000000';
    context.beginPath();
    context.arc(x, y, Math.max(0.5, brushSize.value / 2), 0, Math.PI * 2);
    context.fill();
    version.value += 1;
  };

  const paintLine = (from: { x: number; y: number }, to: { x: number; y: number }, keep: boolean) => {
    const context = ensureMask();
    if (!context) return;
    context.strokeStyle = keep ? '#ffffff' : '#000000';
    context.lineWidth = Math.max(1, brushSize.value);
    context.lineCap = 'round';
    context.beginPath();
    context.moveTo(from.x, from.y);
    context.lineTo(to.x, to.y);
    context.stroke();
    version.value += 1;
  };

  const paintRect = (from: { x: number; y: number }, to: { x: number; y: number }, keep: boolean) => {
    const context = ensureMask();
    if (!context) return;
    context.fillStyle = keep ? '#ffffff' : '#000000';
    context.fillRect(
      Math.min(from.x, to.x), Math.min(from.y, to.y),
      Math.abs(to.x - from.x), Math.abs(to.y - from.y),
    );
    version.value += 1;
  };

  /**
   * 魔棒：从点击处做扫描线区域生长，颜色距离 ≤ tolerance 的连通像素整块保留/擦除。
   * 这就是「点一下去掉背景」的实现，纯前端、无需模型。
   */
  const wandPick = async (x: number, y: number, keep: boolean) => {
    const context = ensureMask();
    if (!context || !maskCanvas) return;
    if (!(await loadSource()) || !sourceCanvas) return;
    const sourceCtx = sourceCanvas.getContext('2d', { willReadFrequently: true });
    if (!sourceCtx) return;
    const width = maskCanvas.width;
    const height = maskCanvas.height;
    const px = Math.round(x);
    const py = Math.round(y);
    if (px < 0 || py < 0 || px >= width || py >= height) return;
    const source = sourceCtx.getImageData(0, 0, width, height).data;
    const mask = context.getImageData(0, 0, width, height);
    const start = (py * width + px) * 4;
    const target = [source[start], source[start + 1], source[start + 2]];
    // 阈值按 RGB 欧氏距离折算（tolerance 0..1 → 0..441）。
    const limit = tolerance.value * 441;
    const visited = new Uint8Array(width * height);
    const value = keep ? 255 : 0;
    const stack: number[] = [py * width + px];
    visited[py * width + px] = 1;
    while (stack.length) {
      const index = stack.pop()!;
      const column = index % width;
      const row = (index - column) / width;
      // 向左扫这一行
      let left = column;
      while (left > 0) {
        const probe = (row * width + left - 1) * 4;
        if (visited[row * width + left - 1]) break;
        if (Math.abs(source[probe] - target[0]) + Math.abs(source[probe + 1] - target[1]) + Math.abs(source[probe + 2] - target[2]) > limit) break;
        left -= 1;
      }
      // 向右扫这一行
      let right = column;
      while (right < width - 1) {
        const probe = (row * width + right + 1) * 4;
        if (visited[row * width + right + 1]) break;
        if (Math.abs(source[probe] - target[0]) + Math.abs(source[probe + 1] - target[1]) + Math.abs(source[probe + 2] - target[2]) > limit) break;
        right += 1;
      }
      for (let column2 = left; column2 <= right; column2 += 1) {
        const linear = row * width + column2;
        visited[linear] = 1;
        const offset = linear * 4;
        mask.data[offset] = value;
        mask.data[offset + 1] = value;
        mask.data[offset + 2] = value;
        if (row > 0 && !visited[linear - width]) stack.push(linear - width);
        if (row < height - 1 && !visited[linear + width]) stack.push(linear + width);
      }
    }
    context.putImageData(mask, 0, 0);
    version.value += 1;
  };

  /** 把遮罩作为半透明压暗层画进预览，用户能直接看到「不会画」的部分。 */
  const renderOverlay = (
    ctx: CanvasRenderingContext2D,
    transform: { offsetX: number; offsetY: number; scale: number },
  ) => {
    if (!enabled.value || !maskCanvas) return;
    ctx.save();
    ctx.globalAlpha = 0.5;
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(maskCanvas, transform.offsetX, transform.offsetY, maskCanvas.width * transform.scale, maskCanvas.height * transform.scale);
    ctx.restore();
  };

  const exportPng = (): string | undefined => {
    if (!enabled.value || !maskCanvas) return undefined;
    return maskCanvas.toDataURL('image/png');
  };

  const hasContent = () => {
    if (!maskCanvas || !maskCtx) return false;
    const image = maskCtx.getImageData(0, 0, maskCanvas.width, maskCanvas.height).data;
    for (let index = 0; index < image.length; index += 4) {
      if (image[index] < 128) return true;
    }
    return false;
  };

  // 换了张图（尺寸变了）就重建遮罩，避免把上一张的选区套在新图上。
  watch(() => [plan.value?.width, plan.value?.height], () => {
    if (enabled.value) ensureMask();
    sourceCanvas = null;
  });

  return {
    enabled,
    tool,
    brushSize,
    tolerance,
    version,
    size,
    ensureMask,
    resetMask,
    invertMask,
    paintCircle,
    paintLine,
    paintRect,
    wandPick,
    renderOverlay,
    exportPng,
    hasContent,
  };
}
