import { describe, expect, it } from 'vitest';
import {
  GENERAL_MIDI_GROUPS,
  GENERAL_MIDI_INSTRUMENTS,
  GeneralMidiSynth,
  getGeneralMidiInstrumentName,
  parseGeneralMidi,
  type MidiNote,
} from './generalMidi';

const midiWithViolinAndDrums = new Uint8Array([
  0x4d, 0x54, 0x68, 0x64, 0x00, 0x00, 0x00, 0x06,
  0x00, 0x00, 0x00, 0x01, 0x00, 0x60,
  0x4d, 0x54, 0x72, 0x6b, 0x00, 0x00, 0x00, 0x17,
  0x00, 0xc0, 0x28,
  0x00, 0x90, 0x3c, 0x64,
  0x60, 0x80, 0x3c, 0x00,
  0x00, 0x99, 0x24, 0x7f,
  0x30, 0x89, 0x24, 0x00,
  0x00, 0xff, 0x2f, 0x00,
]);

describe('General MIDI support', () => {
  it('exposes all 128 instruments in 16 standard groups', () => {
    expect(GENERAL_MIDI_INSTRUMENTS).toHaveLength(128);
    expect(GENERAL_MIDI_GROUPS).toHaveLength(16);
    expect(GENERAL_MIDI_GROUPS.every((group) => group.instruments.length === 8)).toBe(true);
    expect(getGeneralMidiInstrumentName(0)).toBe('大钢琴');
    expect(getGeneralMidiInstrumentName(40)).toBe('小提琴');
  });

  it('keeps source program changes and channel 10 percussion', () => {
    const parsed = parseGeneralMidi(midiWithViolinAndDrums);

    expect(parsed.programs).toEqual([40]);
    expect(parsed.hasPercussion).toBe(true);
    expect(parsed.notes).toHaveLength(2);
    expect(parsed.notes[0]).toMatchObject({ note: 60, channel: 0, program: 40 });
    expect(parsed.notes[0].durationMs).toBeCloseTo(500);
    expect(parsed.notes[1]).toMatchObject({ note: 36, channel: 9 });
    expect(parsed.controlChanges).toEqual([]);
    expect(parsed.hasSustainPedal).toBe(false);
  });

  it('extends note duration through a channel sustain pedal cycle', () => {
    const midi = new Uint8Array([
      0x4d, 0x54, 0x68, 0x64, 0x00, 0x00, 0x00, 0x06,
      0x00, 0x00, 0x00, 0x01, 0x00, 0x60,
      0x4d, 0x54, 0x72, 0x6b, 0x00, 0x00, 0x00, 0x17,
      0x00, 0xc0, 0x00,
      0x00, 0x90, 0x3c, 0x64,
      0x30, 0xb0, 0x40, 0x7f,
      0x30, 0x80, 0x3c, 0x00,
      0x30, 0xb0, 0x40, 0x00,
      0x00, 0xff, 0x2f, 0x00,
    ]);

    const parsed = parseGeneralMidi(midi);
    expect(parsed.hasSustainPedal).toBe(true);
    expect(parsed.controlChanges).toHaveLength(2);
    expect(parsed.controlChanges.map((event) => event.controller)).toEqual([64, 64]);
    expect(parsed.notes[0].durationMs).toBeCloseTo(750, 0);
  });
});

/** 最小 WebAudio 桩：只记录发声图是怎么搭的，不做真实音频。 */
type ParamCall = { method: string; value: number; time: number };

const makeParam = (calls: ParamCall[], name: string) => {
  const record = (method: string) => (value: number, time: number) => {
    calls.push({ method, value, time });
    // 真实的 AudioParam 在 setValueAtTime 之后能被读到当前值，桩也要这样，
    // 否则断言只能看到 0。
    if (method === 'setValueAtTime') param.value = value;
    return param;
  };
  const param = {
    value: 0,
    setValueAtTime: record('setValueAtTime'),
    linearRampToValueAtTime: record('linearRampToValueAtTime'),
    exponentialRampToValueAtTime: record('exponentialRampToValueAtTime'),
    cancelScheduledValues: () => param,
  };
  return param;
};

const createAudioStub = () => {
  const calls: ParamCall[] = [];
  const oscillators: Array<{ type: string; frequency: { value: number }; detune: unknown; startedAt: number | null; stoppedAt: number | null }> = [];
  const gains: Array<Record<string, unknown>> = [];
  const node = (kind: string) => ({
    kind,
    connect: (target: unknown) => target,
    disconnect: () => undefined,
    addEventListener: () => undefined,
  });
  const context = {
    sampleRate: 48000,
    currentTime: 0,
    state: 'running',
    destination: node('destination'),
    createOscillator: () => {
      const frequency = makeParam(calls, 'frequency');
      const oscillator = {
        ...node('oscillator'),
        type: 'sine' as string,
        frequency,
        detune: makeParam(calls, 'detune'),
        startedAt: null as number | null,
        stoppedAt: null as number | null,
        start: (time: number) => { oscillator.startedAt = time; },
        stop: (time: number) => { oscillator.stoppedAt = time; },
      };
      oscillators.push(oscillator as never);
      return oscillator;
    },
    createGain: () => {
      const gainNode = { ...node('gain'), gain: makeParam(calls, 'gain') };
      gains.push(gainNode as never);
      return gainNode;
    },
    createBiquadFilter: () => ({ ...node('filter'), type: 'lowpass', frequency: makeParam(calls, 'filterFrequency'), Q: makeParam(calls, 'Q') }),
    createStereoPanner: () => ({ ...node('panner'), pan: makeParam(calls, 'pan') }),
    createBufferSource: () => ({ ...node('bufferSource'), buffer: null, playbackRate: makeParam(calls, 'playbackRate'), start: () => undefined, stop: () => undefined }),
    createConvolver: () => ({ ...node('convolver'), buffer: null }),
    createBuffer: (channels: number, length: number) => ({
      length,
      getChannelData: () => new Float32Array(length),
    }),
  };
  return { context: context as unknown as AudioContext, calls, oscillators, gains };
};

const note = (overrides: Partial<MidiNote>): MidiNote => ({
  timeMs: 0,
  durationMs: 500,
  note: 60,
  velocity: 0.6,
  channel: 0,
  program: 0,
  ...overrides,
});

describe('内置播放器的钢琴发声模型', () => {
  it('用多个非谐分音而不是单个三角波，且分音频率略高于整数倍', () => {
    const { context, oscillators } = createAudioStub();
    const synth = new GeneralMidiSynth(context, context.destination);
    synth.schedule(note({ note: 45, program: 0 }), 0, 0.5, null);

    const fundamentals = oscillators.filter(oscillator => oscillator.type === 'sine');
    expect(fundamentals.length).toBeGreaterThan(4);
    const f0 = fundamentals[0].frequency.value;
    const second = fundamentals[1].frequency.value;
    // 理想弦是 2·f0；真钢因为弦刚度会偏高，所以必须严格大于 2·f0。
    expect(second).toBeGreaterThan(f0 * 2);
    expect(second).toBeLessThan(f0 * 2.2);
  });

  it('低音分音更多、高音分音更少', () => {
    const bass = createAudioStub();
    new GeneralMidiSynth(bass.context, bass.context.destination).schedule(note({ note: 30 }), 0, 1, null);
    const treble = createAudioStub();
    new GeneralMidiSynth(treble.context, treble.context.destination).schedule(note({ note: 100 }), 0, 1, null);
    const sineCount = (set: typeof bass.oscillators) => set.filter(oscillator => oscillator.type === 'sine').length;
    expect(sineCount(bass.oscillators)).toBeGreaterThan(sineCount(treble.oscillators));
  });

  it('力度同时决定音量与亮度', () => {
    const soft = createAudioStub();
    new GeneralMidiSynth(soft.context, soft.context.destination).schedule(note({ velocity: 0.2 }), 0, 0.5, null);
    const loud = createAudioStub();
    new GeneralMidiSynth(loud.context, loud.context.destination).schedule(note({ velocity: 1 }), 0, 0.5, null);
    const peakOf = (set: typeof soft.calls) => Math.max(...set.filter(call => call.method === 'linearRampToValueAtTime').map(call => call.value));
    expect(peakOf(loud.calls)).toBeGreaterThan(peakOf(soft.calls) * 2);
  });

  it('松键后在制音器处收掉，不会像风琴一样无限延音', () => {
    const { context, calls } = createAudioStub();
    new GeneralMidiSynth(context, context.destination).schedule(note({ note: 60 }), 0, 0.25, null);
    const finalRamps = calls.filter(call => call.method === 'exponentialRampToValueAtTime' && call.value === 0.0001);
    expect(finalRamps.length).toBeGreaterThan(0);
    // 制音器 = 音符时长 + 80ms，之后不允许还有更晚的包络排程。
    const damperAt = 0.25 + 0.08;
    expect(Math.max(...calls.map(call => call.time))).toBeLessThanOrEqual(damperAt + 0.05);
  });

  it('非钢琴程序仍走各自的家族音色', () => {
    const { context, oscillators } = createAudioStub();
    new GeneralMidiSynth(context, context.destination).schedule(note({ note: 64, program: 40 }), 0, 0.5, null);
    expect(oscillators.length).toBe(2);
    expect(oscillators.every(oscillator => oscillator.type === 'sawtooth')).toBe(true);
  });
});
