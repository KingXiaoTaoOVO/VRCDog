export type MidiNote = {
  timeMs: number;
  durationMs: number;
  note: number;
  velocity: number;
  channel: number;
  program: number;
};

export type MidiParseResult = {
  notes: MidiNote[];
  programs: number[];
  hasPercussion: boolean;
};

export type GeneralMidiGroup = {
  name: string;
  instruments: Array<{ program: number; name: string }>;
};

const GENERAL_MIDI_GROUP_NAMES = [
  '钢琴', '半音阶打击乐', '风琴', '吉他', '贝斯', '弦乐', '合奏', '铜管乐',
  '簧管乐', '管乐', '合成主音', '合成音垫', '合成效果', '民族乐器', '打击乐', '音效',
];

export const GENERAL_MIDI_INSTRUMENTS = [
  '大钢琴', '明亮钢琴', '电声大钢琴', '酒吧钢琴', '电钢琴 1', '电钢琴 2', '羽管键琴', '击弦古钢琴',
  '钢片琴', '钟琴', '八音盒', '颤音琴', '马林巴', '木琴', '管钟', '扬琴',
  '拉杆风琴', '打击风琴', '摇滚风琴', '教堂风琴', '簧风琴', '手风琴', '口琴', '探戈手风琴',
  '尼龙弦吉他', '钢弦吉他', '爵士电吉他', '清音电吉他', '闷音电吉他', '过载吉他', '失真吉他', '吉他泛音',
  '原声贝斯', '指弹贝斯', '拨片贝斯', '无品贝斯', '击弦贝斯 1', '击弦贝斯 2', '合成贝斯 1', '合成贝斯 2',
  '小提琴', '中提琴', '大提琴', '低音提琴', '颤弓弦乐', '拨奏弦乐', '竖琴', '定音鼓',
  '弦乐合奏 1', '弦乐合奏 2', '合成弦乐 1', '合成弦乐 2', '人声合唱', '人声吟唱', '合成人声', '管弦乐齐奏',
  '小号', '长号', '大号', '弱音小号', '圆号', '铜管乐组', '合成铜管 1', '合成铜管 2',
  '高音萨克斯', '中音萨克斯', '次中音萨克斯', '上低音萨克斯', '双簧管', '英国管', '巴松管', '单簧管',
  '短笛', '长笛', '竖笛', '排箫', '吹瓶声', '尺八', '口哨', '陶笛',
  '方波主音', '锯齿波主音', '汽笛主音', '短笛主音', '吉他主音', '人声主音', '五度主音', '贝斯加主音',
  '新时代音垫', '温暖音垫', '复音合成音垫', '合唱音垫', '弓弦音垫', '金属音垫', '光环音垫', '扫掠音垫',
  '雨声效果', '电影音效', '水晶音效', '氛围音效', '明亮音效', '奇幻音效', '回声效果', '科幻音效',
  '西塔琴', '班卓琴', '三味线', '古筝', '卡林巴', '风笛', '民谣提琴', '唢呐',
  '叮当铃', '阿哥哥鼓', '钢鼓', '木鱼', '太鼓', '旋律通鼓', '合成鼓', '反向钹',
  '吉他品噪', '呼吸声', '海浪声', '啁啾音效', '电话铃', '直升机', '掌声', '枪声',
] as const;

export const GENERAL_MIDI_GROUPS: GeneralMidiGroup[] = GENERAL_MIDI_GROUP_NAMES.map((name, groupIndex) => ({
  name,
  instruments: GENERAL_MIDI_INSTRUMENTS
    .slice(groupIndex * 8, groupIndex * 8 + 8)
    .map((instrumentName, index) => ({ program: groupIndex * 8 + index, name: instrumentName })),
}));

export const getGeneralMidiInstrumentName = (program: number) => (
  GENERAL_MIDI_INSTRUMENTS[Math.min(127, Math.max(0, Math.round(program)))] || GENERAL_MIDI_INSTRUMENTS[0]
);

const readText = (bytes: Uint8Array, offset: number, length: number) => (
  Array.from(bytes.slice(offset, offset + length)).map((byte) => String.fromCharCode(byte)).join('')
);

const readU16 = (bytes: Uint8Array, offset: number) => (bytes[offset] << 8) | bytes[offset + 1];
const readU32 = (bytes: Uint8Array, offset: number) => (
  (bytes[offset] << 24) |
  (bytes[offset + 1] << 16) |
  (bytes[offset + 2] << 8) |
  bytes[offset + 3]
) >>> 0;

const readVarLen = (bytes: Uint8Array, offset: number) => {
  let value = 0;
  let cursor = offset;
  for (let i = 0; i < 4 && cursor < bytes.length; i += 1) {
    const current = bytes[cursor++];
    value = (value << 7) | (current & 0x7f);
    if ((current & 0x80) === 0) break;
  }
  return { value, offset: cursor };
};

type MidiTrackRange = { start: number; end: number };
type MidiTempoPoint = { tick: number; tempo: number };

const collectMidiTracks = (bytes: Uint8Array, headerLength: number, trackCount: number) => {
  const tracks: MidiTrackRange[] = [];
  let offset = 8 + headerLength;
  for (let index = 0; index < trackCount && offset + 8 <= bytes.length; index += 1) {
    if (readText(bytes, offset, 4) !== 'MTrk') break;
    const trackLength = readU32(bytes, offset + 4);
    const start = offset + 8;
    const end = Math.min(start + trackLength, bytes.length);
    tracks.push({ start, end });
    offset = start + trackLength;
  }
  return tracks;
};

const walkMidiTrack = (
  bytes: Uint8Array,
  track: MidiTrackRange,
  handlers: {
    meta?: (tick: number, metaType: number, offset: number, length: number) => void;
    midi?: (tick: number, statusByte: number, data1: number, data2: number) => void;
  },
) => {
  let offset = track.start;
  let tick = 0;
  let runningStatus = 0;

  while (offset < track.end) {
    const delta = readVarLen(bytes, offset);
    tick += delta.value;
    offset = delta.offset;
    if (offset >= track.end) break;

    let statusByte = bytes[offset++];
    if (statusByte < 0x80) {
      if (!runningStatus) break;
      offset -= 1;
      statusByte = runningStatus;
    } else if (statusByte < 0xf0) {
      runningStatus = statusByte;
    }

    if (statusByte === 0xff) {
      const metaType = bytes[offset++] ?? 0;
      const lengthInfo = readVarLen(bytes, offset);
      offset = lengthInfo.offset;
      handlers.meta?.(tick, metaType, offset, lengthInfo.value);
      offset = Math.min(offset + lengthInfo.value, track.end);
      continue;
    }

    if (statusByte === 0xf0 || statusByte === 0xf7) {
      const lengthInfo = readVarLen(bytes, offset);
      offset = Math.min(lengthInfo.offset + lengthInfo.value, track.end);
      runningStatus = 0;
      continue;
    }

    const type = statusByte & 0xf0;
    const dataLength = type === 0xc0 || type === 0xd0 ? 1 : 2;
    const data1 = bytes[offset++] ?? 0;
    const data2 = dataLength === 2 ? (bytes[offset++] ?? 0) : 0;
    handlers.midi?.(tick, statusByte, data1, data2);
  }
};

const tickToMs = (tick: number, tempoMap: MidiTempoPoint[], ticksPerBeat: number) => {
  let micros = 0;
  let lastTick = 0;
  let tempo = 500000;
  for (const point of tempoMap) {
    if (point.tick > tick) break;
    if (point.tick > lastTick) {
      micros += ((point.tick - lastTick) * tempo) / ticksPerBeat;
      lastTick = point.tick;
    }
    tempo = point.tempo;
  }
  return (micros + ((tick - lastTick) * tempo) / ticksPerBeat) / 1000;
};

export const parseGeneralMidi = (bytes: Uint8Array): MidiParseResult => {
  if (bytes.length < 14 || readText(bytes, 0, 4) !== 'MThd') throw new Error('不是有效的 MIDI 文件');
  const headerLength = readU32(bytes, 4);
  const trackCount = readU16(bytes, 10);
  const division = readU16(bytes, 12);
  if ((division & 0x8000) !== 0) throw new Error('暂不支持 SMPTE 时间格式 MIDI');

  const ticksPerBeat = Math.max(1, division);
  const tracks = collectMidiTracks(bytes, headerLength, trackCount);
  if (!tracks.length) throw new Error('MIDI 文件中没有有效音轨');

  const tempoMap: MidiTempoPoint[] = [{ tick: 0, tempo: 500000 }];
  for (const track of tracks) {
    walkMidiTrack(bytes, track, {
      meta: (tick, metaType, offset, length) => {
        if (metaType === 0x51 && length === 3 && offset + 3 <= track.end) {
          tempoMap.push({ tick, tempo: (bytes[offset] << 16) | (bytes[offset + 1] << 8) | bytes[offset + 2] });
        }
      },
    });
  }
  tempoMap.sort((a, b) => a.tick - b.tick);
  const compactTempoMap = tempoMap.filter((point, index) => !tempoMap[index + 1] || tempoMap[index + 1].tick !== point.tick);

  const notes: MidiNote[] = [];
  const usedPrograms = new Set<number>();
  let hasPercussion = false;

  for (const track of tracks) {
    const channelPrograms = new Array<number>(16).fill(0);
    const openNotes = new Map<number, Array<{ timeMs: number; velocity: number; program: number }>>();

    walkMidiTrack(bytes, track, {
      midi: (tick, statusByte, data1, data2) => {
        const type = statusByte & 0xf0;
        const channel = statusByte & 0x0f;
        if (type === 0xc0) {
          channelPrograms[channel] = Math.min(127, data1);
          return;
        }

        const key = (channel << 8) | data1;
        const eventTimeMs = tickToMs(tick, compactTempoMap, ticksPerBeat);
        if (type === 0x90 && data2 > 0) {
          const program = channelPrograms[channel];
          const stack = openNotes.get(key) || [];
          stack.push({ timeMs: eventTimeMs, velocity: data2 / 127, program });
          openNotes.set(key, stack);
          if (channel === 9) hasPercussion = true;
          else usedPrograms.add(program);
        } else if (type === 0x80 || (type === 0x90 && data2 === 0)) {
          const stack = openNotes.get(key);
          const start = stack?.shift();
          if (start) {
            notes.push({
              timeMs: start.timeMs,
              durationMs: Math.max(90, eventTimeMs - start.timeMs),
              note: data1,
              velocity: start.velocity,
              channel,
              program: start.program,
            });
          }
        }
      },
    });

    for (const [key, stack] of openNotes.entries()) {
      for (const start of stack) {
        notes.push({
          timeMs: start.timeMs,
          durationMs: 220,
          note: key & 0xff,
          velocity: start.velocity,
          channel: (key >> 8) & 0x0f,
          program: start.program,
        });
      }
    }
  }

  notes.sort((a, b) => a.timeMs - b.timeMs);
  if (!notes.length) throw new Error('这个 MIDI 没有可试听的音符');
  return { notes, programs: Array.from(usedPrograms).sort((a, b) => a - b), hasPercussion };
};

type OscillatorLayer = {
  type: OscillatorType;
  ratio: number;
  gain: number;
  detune?: number;
};

type SynthProfile = {
  layers: OscillatorLayer[];
  attack: number;
  decay: number;
  sustain: number;
  release: number;
  maxDuration: number;
  filterFrequency: number;
  filterQ: number;
  level: number;
};

const FAMILY_PROFILES: SynthProfile[] = [
  { layers: [{ type: 'triangle', ratio: 1, gain: 1 }, { type: 'sine', ratio: 2, gain: 0.28 }], attack: 0.006, decay: 0.8, sustain: 0.2, release: 0.32, maxDuration: 7, filterFrequency: 6200, filterQ: 0.7, level: 0.9 },
  { layers: [{ type: 'sine', ratio: 1, gain: 1 }, { type: 'sine', ratio: 3, gain: 0.42 }], attack: 0.003, decay: 0.65, sustain: 0.05, release: 0.18, maxDuration: 3, filterFrequency: 9000, filterQ: 1.2, level: 0.72 },
  { layers: [{ type: 'sine', ratio: 1, gain: 1 }, { type: 'square', ratio: 2, gain: 0.22 }], attack: 0.025, decay: 0.12, sustain: 0.8, release: 0.2, maxDuration: 12, filterFrequency: 5200, filterQ: 0.5, level: 0.64 },
  { layers: [{ type: 'triangle', ratio: 1, gain: 1 }, { type: 'sawtooth', ratio: 2, gain: 0.14 }], attack: 0.004, decay: 0.5, sustain: 0.18, release: 0.2, maxDuration: 5, filterFrequency: 4200, filterQ: 1.4, level: 0.76 },
  { layers: [{ type: 'triangle', ratio: 1, gain: 1 }, { type: 'square', ratio: 0.5, gain: 0.16 }], attack: 0.008, decay: 0.28, sustain: 0.55, release: 0.18, maxDuration: 6, filterFrequency: 1350, filterQ: 1.1, level: 0.92 },
  { layers: [{ type: 'sawtooth', ratio: 1, gain: 0.62, detune: -5 }, { type: 'sawtooth', ratio: 1, gain: 0.62, detune: 5 }], attack: 0.075, decay: 0.35, sustain: 0.72, release: 0.48, maxDuration: 12, filterFrequency: 3300, filterQ: 0.8, level: 0.55 },
  { layers: [{ type: 'sawtooth', ratio: 1, gain: 0.55, detune: -8 }, { type: 'triangle', ratio: 1, gain: 0.75, detune: 8 }], attack: 0.14, decay: 0.4, sustain: 0.68, release: 0.65, maxDuration: 14, filterFrequency: 3800, filterQ: 0.7, level: 0.5 },
  { layers: [{ type: 'sawtooth', ratio: 1, gain: 0.8 }, { type: 'square', ratio: 1, gain: 0.25 }], attack: 0.025, decay: 0.22, sustain: 0.7, release: 0.23, maxDuration: 8, filterFrequency: 2500, filterQ: 1.8, level: 0.58 },
  { layers: [{ type: 'square', ratio: 1, gain: 0.72 }, { type: 'triangle', ratio: 2, gain: 0.18 }], attack: 0.035, decay: 0.2, sustain: 0.66, release: 0.2, maxDuration: 9, filterFrequency: 2900, filterQ: 2.4, level: 0.52 },
  { layers: [{ type: 'sine', ratio: 1, gain: 1 }, { type: 'triangle', ratio: 2, gain: 0.2 }], attack: 0.045, decay: 0.18, sustain: 0.78, release: 0.28, maxDuration: 10, filterFrequency: 6400, filterQ: 0.9, level: 0.68 },
  { layers: [{ type: 'sawtooth', ratio: 1, gain: 0.82 }, { type: 'square', ratio: 2, gain: 0.18 }], attack: 0.008, decay: 0.16, sustain: 0.7, release: 0.16, maxDuration: 8, filterFrequency: 5200, filterQ: 2.3, level: 0.52 },
  { layers: [{ type: 'triangle', ratio: 1, gain: 0.78, detune: -7 }, { type: 'sawtooth', ratio: 1, gain: 0.35, detune: 7 }], attack: 0.3, decay: 0.6, sustain: 0.65, release: 0.95, maxDuration: 16, filterFrequency: 2600, filterQ: 0.8, level: 0.46 },
  { layers: [{ type: 'sine', ratio: 1, gain: 0.8 }, { type: 'sine', ratio: 1.5, gain: 0.38, detune: 12 }], attack: 0.12, decay: 0.5, sustain: 0.52, release: 0.8, maxDuration: 12, filterFrequency: 4800, filterQ: 2.8, level: 0.48 },
  { layers: [{ type: 'triangle', ratio: 1, gain: 0.88 }, { type: 'sine', ratio: 2, gain: 0.3 }], attack: 0.006, decay: 0.6, sustain: 0.16, release: 0.28, maxDuration: 5, filterFrequency: 4400, filterQ: 1.3, level: 0.7 },
  { layers: [{ type: 'sine', ratio: 1, gain: 0.75 }, { type: 'square', ratio: 2.5, gain: 0.2 }], attack: 0.003, decay: 0.34, sustain: 0.08, release: 0.18, maxDuration: 3, filterFrequency: 6600, filterQ: 1.1, level: 0.68 },
  { layers: [{ type: 'sine', ratio: 1, gain: 0.72 }, { type: 'sawtooth', ratio: 0.5, gain: 0.18 }], attack: 0.015, decay: 0.45, sustain: 0.15, release: 0.45, maxDuration: 5, filterFrequency: 3600, filterQ: 2, level: 0.58 },
];

const midiNoteFrequency = (note: number) => 440 * Math.pow(2, (note - 69) / 12);

export class GeneralMidiSynth {
  private readonly scheduledNodes = new Set<AudioNode>();
  private noiseBuffer: AudioBuffer | null = null;

  constructor(private readonly context: AudioContext, private readonly output: AudioNode) {}

  stopAll() {
    for (const node of this.scheduledNodes) {
      try {
        const source = node as AudioScheduledSourceNode;
        if (typeof source.stop === 'function') source.stop();
        node.disconnect();
      } catch {
        // A scheduled source may already have ended.
      }
    }
    this.scheduledNodes.clear();
  }

  schedule(note: MidiNote, startAt: number, duration: number, overrideProgram: number | null) {
    if (note.channel === 9) this.scheduleDrum(note, startAt);
    else this.scheduleInstrument(note, startAt, duration, overrideProgram ?? note.program);
  }

  private trackSource(source: AudioScheduledSourceNode, nodes: AudioNode[]) {
    for (const node of nodes) this.scheduledNodes.add(node);
    source.addEventListener('ended', () => {
      for (const node of nodes) {
        this.scheduledNodes.delete(node);
        try { node.disconnect(); } catch { /* Already disconnected. */ }
      }
    }, { once: true });
  }

  private scheduleInstrument(note: MidiNote, startAt: number, duration: number, program: number) {
    const family = Math.floor(Math.min(127, Math.max(0, program)) / 8);
    const variation = program % 8;
    const profile = FAMILY_PROFILES[family];
    const voiceGain = this.context.createGain();
    const filter = this.context.createBiquadFilter();
    const panner = this.context.createStereoPanner();
    const peak = Math.max(0.008, 0.16 * note.velocity * profile.level);
    const audibleDuration = Math.max(0.05, Math.min(duration, profile.maxDuration));
    const attackEnd = startAt + Math.min(profile.attack, audibleDuration * 0.35);
    const decayEnd = Math.min(startAt + audibleDuration, attackEnd + profile.decay);
    const releaseStart = Math.max(decayEnd, startAt + audibleDuration);
    const endAt = releaseStart + profile.release;

    filter.type = 'lowpass';
    filter.frequency.setValueAtTime(Math.min(12000, profile.filterFrequency * (0.82 + variation * 0.055)), startAt);
    filter.Q.setValueAtTime(profile.filterQ, startAt);
    panner.pan.setValueAtTime(((note.channel % 4) - 1.5) * 0.1, startAt);
    voiceGain.gain.setValueAtTime(0.0001, startAt);
    voiceGain.gain.linearRampToValueAtTime(peak, attackEnd);
    voiceGain.gain.exponentialRampToValueAtTime(Math.max(0.0001, peak * profile.sustain), decayEnd);
    voiceGain.gain.setValueAtTime(Math.max(0.0001, peak * profile.sustain), releaseStart);
    voiceGain.gain.exponentialRampToValueAtTime(0.0001, endAt);
    voiceGain.connect(filter).connect(panner).connect(this.output);

    const sources: OscillatorNode[] = [];
    const frequency = midiNoteFrequency(note.note);
    for (const layer of profile.layers) {
      const oscillator = this.context.createOscillator();
      const layerGain = this.context.createGain();
      oscillator.type = layer.type;
      oscillator.frequency.setValueAtTime(frequency * layer.ratio, startAt);
      oscillator.detune.setValueAtTime((layer.detune || 0) + (variation - 3.5) * 0.7, startAt);
      layerGain.gain.setValueAtTime(layer.gain, startAt);
      oscillator.connect(layerGain).connect(voiceGain);
      oscillator.start(startAt);
      oscillator.stop(endAt + 0.03);
      sources.push(oscillator);
      this.trackSource(oscillator, [oscillator, layerGain]);
    }
    if (sources[0]) this.trackSource(sources[0], [voiceGain, filter, panner]);
  }

  private getNoiseBuffer() {
    if (this.noiseBuffer) return this.noiseBuffer;
    const length = this.context.sampleRate * 2;
    const buffer = this.context.createBuffer(1, length, this.context.sampleRate);
    const data = buffer.getChannelData(0);
    for (let index = 0; index < length; index += 1) data[index] = Math.random() * 2 - 1;
    this.noiseBuffer = buffer;
    return buffer;
  }

  private scheduleDrum(note: MidiNote, startAt: number) {
    const velocity = Math.max(0.08, note.velocity);
    if (note.note === 35 || note.note === 36) {
      const oscillator = this.context.createOscillator();
      const gain = this.context.createGain();
      oscillator.type = 'sine';
      oscillator.frequency.setValueAtTime(145, startAt);
      oscillator.frequency.exponentialRampToValueAtTime(45, startAt + 0.18);
      gain.gain.setValueAtTime(0.3 * velocity, startAt);
      gain.gain.exponentialRampToValueAtTime(0.0001, startAt + 0.32);
      oscillator.connect(gain).connect(this.output);
      oscillator.start(startAt);
      oscillator.stop(startAt + 0.34);
      this.trackSource(oscillator, [oscillator, gain]);
      return;
    }

    const source = this.context.createBufferSource();
    const filter = this.context.createBiquadFilter();
    const gain = this.context.createGain();
    const isHat = [42, 44, 46].includes(note.note);
    const isCymbal = [49, 51, 52, 55, 57, 59].includes(note.note);
    const isTom = note.note >= 41 && note.note <= 50 && !isHat && !isCymbal;
    const duration = isCymbal ? 0.95 : isHat ? (note.note === 46 ? 0.32 : 0.1) : isTom ? 0.38 : 0.24;
    source.buffer = this.getNoiseBuffer();
    filter.type = isHat || isCymbal ? 'highpass' : isTom ? 'lowpass' : 'bandpass';
    filter.frequency.setValueAtTime(isHat ? 7200 : isCymbal ? 4700 : isTom ? 1100 : 1900, startAt);
    filter.Q.setValueAtTime(isTom ? 1.5 : 0.8, startAt);
    gain.gain.setValueAtTime((isCymbal ? 0.09 : 0.16) * velocity, startAt);
    gain.gain.exponentialRampToValueAtTime(0.0001, startAt + duration);
    source.connect(filter).connect(gain).connect(this.output);
    source.start(startAt, Math.random());
    source.stop(startAt + duration);
    this.trackSource(source, [source, filter, gain]);

    if (isTom) {
      const oscillator = this.context.createOscillator();
      const toneGain = this.context.createGain();
      oscillator.type = 'triangle';
      oscillator.frequency.setValueAtTime(90 + (note.note - 41) * 18, startAt);
      toneGain.gain.setValueAtTime(0.18 * velocity, startAt);
      toneGain.gain.exponentialRampToValueAtTime(0.0001, startAt + duration);
      oscillator.connect(toneGain).connect(this.output);
      oscillator.start(startAt);
      oscillator.stop(startAt + duration);
      this.trackSource(oscillator, [oscillator, toneGain]);
    }
  }
}
