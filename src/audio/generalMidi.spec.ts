import { describe, expect, it } from 'vitest';
import {
  GENERAL_MIDI_GROUPS,
  GENERAL_MIDI_INSTRUMENTS,
  getGeneralMidiInstrumentName,
  parseGeneralMidi,
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
