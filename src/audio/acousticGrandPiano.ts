/**
 * 高品质真实声学三角钢琴采样音频引擎 (FreePiano / FluidR3_GM 级别)
 * 基于 88 键高精度真实录音采样与 Web Audio 解码缓冲，提供殿堂级真实钢琴音色
 */

const NOTE_NAMES = ['C', 'Db', 'D', 'Eb', 'E', 'F', 'Gb', 'G', 'Ab', 'A', 'Bb', 'B'];

export function midiNoteToName(midi: number): string {
  const octave = Math.floor(midi / 12) - 1;
  return `${NOTE_NAMES[midi % 12]}${octave}`;
}

export class AcousticGrandPianoEngine {
  private static instance: AcousticGrandPianoEngine | null = null;
  private soundfontData: Record<string, string> | null = null;
  private audioBuffers = new Map<number, AudioBuffer>();
  private pendingDecodes = new Map<number, Promise<AudioBuffer | null>>();
  private isLoaded = false;
  private loadPromise: Promise<boolean> | null = null;

  public static getInstance(): AcousticGrandPianoEngine {
    if (!AcousticGrandPianoEngine.instance) {
      AcousticGrandPianoEngine.instance = new AcousticGrandPianoEngine();
    }
    return AcousticGrandPianoEngine.instance;
  }

  public async ensureLoaded(): Promise<boolean> {
    if (this.isLoaded && this.soundfontData) return true;
    if (this.loadPromise) return this.loadPromise;

    this.loadPromise = (async () => {
      // 1. 检查全局是否已由 <script> 注入
      const globalWindow = typeof window !== 'undefined' ? (window as any) : null;
      if (globalWindow?.MIDI?.Soundfont?.acoustic_grand_piano) {
        this.soundfontData = globalWindow.MIDI.Soundfont.acoustic_grand_piano;
        this.isLoaded = true;
        return true;
      }

      // 2. 动态加载脚本文件
      if (typeof document !== 'undefined') {
        const baseUrl = (typeof import.meta !== 'undefined' && (import.meta as any).env && (import.meta as any).env.BASE_URL)
          ? String((import.meta as any).env.BASE_URL)
          : '/';
        const scriptUrl = `${baseUrl.replace(/\/$/, '')}/soundfont/acoustic_grand_piano-mp3.js`;
        const loaded = await new Promise<boolean>((resolve) => {
          const script = document.createElement('script');
          script.src = scriptUrl;
          script.async = true;
          script.onload = () => {
            const data = (window as any)?.MIDI?.Soundfont?.acoustic_grand_piano;
            if (data) {
              this.soundfontData = data;
              this.isLoaded = true;
              resolve(true);
            } else {
              resolve(false);
            }
          };
          script.onerror = () => resolve(false);
          document.head.appendChild(script);
        });
        if (loaded) return true;
      }
      return false;
    })();

    return this.loadPromise;
  }

  public isReady(): boolean {
    return Boolean(this.isLoaded && this.soundfontData);
  }

  public preloadCommonOctaves(context: AudioContext): void {
    const doPreload = () => {
      // 核心区域 C3(48) 到 C6(84) 步长 2 密集预热
      for (let note = 48; note <= 84; note += 2) {
        void this.getNoteBuffer(context, note);
      }
      // 全音域 88 键 (A0=21 到 C8=108) 步长 3 覆盖，确保任意按键在偏移 <=2 范围内必有就绪样本
      for (let note = 21; note <= 108; note += 3) {
        void this.getNoteBuffer(context, note);
      }
    };
    if (!this.soundfontData) {
      void this.ensureLoaded().then((ready) => {
        if (ready) doPreload();
      });
      return;
    }
    doPreload();
  }

  public async getNoteBuffer(context: AudioContext, midiNote: number): Promise<AudioBuffer | null> {
    const clampedNote = Math.min(108, Math.max(21, Math.round(midiNote)));
    const cached = this.audioBuffers.get(clampedNote);
    if (cached) return cached;

    const pending = this.pendingDecodes.get(clampedNote);
    if (pending) return pending;

    const decodePromise = (async () => {
      if (!this.soundfontData) {
        await this.ensureLoaded();
      }
      if (!this.soundfontData) return null;

      const noteName = midiNoteToName(clampedNote);
      const dataUri = this.soundfontData[noteName];
      if (!dataUri) return null;

      try {
        const base64Index = dataUri.indexOf(',');
        const base64 = base64Index >= 0 ? dataUri.slice(base64Index + 1) : dataUri;
        const binary = atob(base64);
        const bytes = new Uint8Array(binary.length);
        for (let i = 0; i < binary.length; i++) {
          bytes[i] = binary.charCodeAt(i);
        }
        const buffer = await context.decodeAudioData(bytes.buffer.slice(0));
        this.audioBuffers.set(clampedNote, buffer);
        return buffer;
      } catch {
        return null;
      } finally {
        this.pendingDecodes.delete(clampedNote);
      }
    })();

    this.pendingDecodes.set(clampedNote, decodePromise);
    return decodePromise;
  }

  public scheduleSampledNote(
    context: AudioContext,
    destination: AudioNode,
    midiNote: number,
    velocity: number,
    startAt: number,
    duration: number,
    onNodeScheduled?: (node: AudioNode) => void,
  ): boolean {
    const clampedNote = Math.min(108, Math.max(21, Math.round(midiNote)));
    let buffer = this.audioBuffers.get(clampedNote);
    let playbackRate = 1.0;

    // 如果当前音符尚未解码完成，但相邻半音已就绪，立即移调复用，零延迟发声
    if (!buffer) {
      void this.getNoteBuffer(context, clampedNote);
      for (const offset of [-1, 1, -2, 2, -3, 3]) {
        const nearby = clampedNote + offset;
        const nearbyBuffer = this.audioBuffers.get(nearby);
        if (nearbyBuffer) {
          buffer = nearbyBuffer;
          playbackRate = Math.pow(2, -offset / 12);
          break;
        }
      }
    }

    if (!buffer) {
      return false;
    }

    try {
      const source = context.createBufferSource();
      source.buffer = buffer;
      source.playbackRate.value = playbackRate;

      const gain = context.createGain();
      const vel = Math.min(1, Math.max(0.05, velocity));
      const peakGain = Math.min(1.2, Math.pow(vel, 1.25) * 0.95);

      const damperAt = startAt + Math.max(0.12, duration);
      gain.gain.setValueAtTime(0.0001, startAt);
      gain.gain.linearRampToValueAtTime(peakGain, startAt + 0.003);

      // 声学钢琴自然延音衰减 (Soundboard Body Resonance)
      const naturalDecay = Math.max(0.8, 5.0 - (clampedNote - 21) * 0.04);
      const decayEnd = Math.min(damperAt, startAt + naturalDecay);
      gain.gain.exponentialRampToValueAtTime(Math.max(0.0001, peakGain * 0.35), decayEnd);

      // 制音器下落 (Damper Release)
      gain.gain.setValueAtTime(Math.max(0.0001, peakGain * 0.3), damperAt);
      gain.gain.exponentialRampToValueAtTime(0.0001, damperAt + 0.16);

      // 声学键盘立体声声场展宽 (Stereo Panning across 88 keys)
      const panner = context.createStereoPanner();
      panner.pan.value = Math.max(-0.55, Math.min(0.55, (clampedNote - 60) / 48));

      source.connect(gain);
      gain.connect(panner);
      panner.connect(destination);

      source.start(startAt);
      source.stop(damperAt + 0.2);

      if (onNodeScheduled) {
        onNodeScheduled(source);
        onNodeScheduled(gain);
        onNodeScheduled(panner);
      }

      source.addEventListener('ended', () => {
        try {
          source.disconnect();
          gain.disconnect();
          panner.disconnect();
        } catch {}
      }, { once: true });

      return true;
    } catch {
      return false;
    }
  }
}
