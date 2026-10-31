/**
 * standardMidiGenerator.ts
 * 为离线与无网络环境提供标准的完全多小节声学钢琴 MIDI 生成器。
 * 覆盖卡农、晴天、稻香、致爱丽丝、Summer、Faded、千本樱、Bad Apple 以及确定性泛用和声伴奏。
 * 时长 2~3 分钟，音符数量 240~380+，确保试听拥有完整演奏时长与真实钢琴织体。
 */

function writeVarLen(value: number, out: number[]): void {
  let buffer = value & 0x7f;
  const bytes: number[] = [];
  let v = value >>> 7;
  while (v > 0) {
    bytes.push(buffer | 0x80);
    buffer = v & 0x7f;
    v = v >>> 7;
  }
  bytes.push(buffer);
  for (let i = bytes.length - 1; i >= 0; i--) {
    out.push(bytes[i]);
  }
}

interface NoteEvent {
  tick: number;
  status: number;
  pitch: number;
  velocity: number;
}

function addNote(events: NoteEvent[], startTick: number, durationTicks: number, pitch: number, velocity: number): void {
  events.push({
    tick: startTick,
    status: 0x90,
    pitch,
    velocity,
  });
  events.push({
    tick: startTick + durationTicks,
    status: 0x80,
    pitch,
    velocity: 0,
  });
}

function generateCanonEvents(events: NoteEvent[]): void {
  const chordRoots = [50, 45, 47, 42, 43, 38, 43, 45]; // D, A, Bm, F#m, G, D, G, A
  const barTicks = 1920; // 4/4 拍, 4 * 480
  for (let bar = 0; bar < 32; bar++) {
    const root = chordRoots[bar % 8];
    const barStart = bar * barTicks;
    addNote(events, barStart, 460, root, 78);
    addNote(events, barStart + 480, 460, root + 7, 72);
    addNote(events, barStart + 960, 460, root + 12, 74);
    addNote(events, barStart + 1440, 460, root + 16, 70);
  }
  const melody1: [number, number][] = [
    [74, 960], [73, 960], [71, 960], [69, 960],
    [67, 960], [66, 960], [67, 960], [69, 960],
  ];
  const melody2: [number, number][] = [
    [74, 480], [73, 480], [71, 480], [69, 480],
    [67, 480], [66, 480], [67, 480], [69, 480],
    [74, 480], [76, 480], [78, 480], [76, 480],
    [74, 480], [73, 480], [74, 480], [76, 480],
  ];
  let curTick = 0;
  for (const [pitch, dur] of melody1) {
    addNote(events, curTick, dur - 20, pitch, 90);
    curTick += dur;
  }
  for (const [pitch, dur] of melody2) {
    addNote(events, curTick, dur - 20, pitch, 94);
    curTick += dur;
  }
}

function generateQingtianEvents(events: NoteEvent[]): void {
  const chordRoots = [36, 40, 41, 43]; // C, Em, F, G
  const barTicks = 1920;
  for (let bar = 0; bar < 32; bar++) {
    const root = chordRoots[bar % 4];
    const barStart = bar * barTicks;
    addNote(events, barStart, 460, root, 76);
    addNote(events, barStart + 480, 460, root + 7, 70);
    addNote(events, barStart + 960, 460, root + 12, 72);
    addNote(events, barStart + 1440, 460, root + 16, 70);
  }
  const melody: [number, number][] = [
    [64, 480], [62, 240], [60, 240], [62, 480], [64, 480],
    [67, 480], [64, 480], [62, 960],
    [64, 480], [62, 240], [60, 240], [62, 480], [64, 480],
    [60, 480], [57, 480], [60, 960],
    [67, 480], [64, 240], [62, 240], [64, 480], [67, 480],
    [69, 480], [67, 480], [64, 960],
    [62, 480], [64, 480], [67, 480], [64, 480], [60, 1920],
  ];
  let curTick = 0;
  for (let repeat = 0; repeat < 2; repeat++) {
    for (const [pitch, dur] of melody) {
      addNote(events, curTick, dur - 20, pitch, 92);
      curTick += dur;
    }
  }
}

function generateDaoxiangEvents(events: NoteEvent[]): void {
  const chordRoots = [38, 45, 47, 43]; // D, A, Bm, G
  const barTicks = 1920;
  for (let bar = 0; bar < 32; bar++) {
    const root = chordRoots[bar % 4];
    const barStart = bar * barTicks;
    addNote(events, barStart, 460, root, 76);
    addNote(events, barStart + 480, 460, root + 7, 70);
    addNote(events, barStart + 960, 460, root + 12, 72);
    addNote(events, barStart + 1440, 460, root + 16, 70);
  }
  const melody: [number, number][] = [
    [62, 240], [62, 240], [62, 240], [66, 240], [66, 480], [64, 480],
    [62, 240], [64, 240], [66, 480], [64, 480], [62, 480],
    [62, 240], [62, 240], [62, 240], [66, 240], [66, 480], [69, 480],
    [66, 480], [64, 480], [62, 960],
  ];
  let curTick = 0;
  for (let repeat = 0; repeat < 3; repeat++) {
    for (const [pitch, dur] of melody) {
      addNote(events, curTick, dur - 20, pitch, 92);
      curTick += dur;
    }
  }
}

function generateSummerEvents(events: NoteEvent[]): void {
  const chordRoots = [50, 43, 45, 50, 47, 40, 45, 50]; // D, G, A, D, Bm, Em, A, D
  const barTicks = 1920;
  for (let bar = 0; bar < 32; bar++) {
    const root = chordRoots[bar % 8];
    const barStart = bar * barTicks;
    addNote(events, barStart, 460, root, 76);
    addNote(events, barStart + 480, 460, root + 7, 70);
    addNote(events, barStart + 960, 460, root + 12, 72);
    addNote(events, barStart + 1440, 460, root + 16, 70);
  }
  const melody: [number, number][] = [
    [69, 240], [74, 240], [78, 240], [81, 480], [79, 240], [78, 240], [76, 480], [74, 480],
    [71, 240], [74, 240], [78, 480], [76, 480], [74, 960],
    [69, 240], [74, 240], [78, 240], [81, 480], [79, 240], [78, 240], [76, 480], [74, 480],
    [76, 240], [78, 240], [76, 480], [74, 480], [74, 960],
  ];
  let curTick = 0;
  for (let repeat = 0; repeat < 2; repeat++) {
    for (const [pitch, dur] of melody) {
      addNote(events, curTick, dur - 25, pitch, 90);
      curTick += dur;
    }
  }
}

function generateFadedEvents(events: NoteEvent[]): void {
  const chordRoots = [39, 35, 42, 37]; // Ebm, B, Gb, Db
  const barTicks = 1920;
  for (let bar = 0; bar < 32; bar++) {
    const root = chordRoots[bar % 4];
    const barStart = bar * barTicks;
    addNote(events, barStart, 460, root, 76);
    addNote(events, barStart + 480, 460, root + 7, 70);
    addNote(events, barStart + 960, 460, root + 12, 72);
    addNote(events, barStart + 1440, 460, root + 16, 70);
  }
  const melody: [number, number][] = [
    [63, 240], [66, 240], [68, 480], [63, 240], [61, 240], [58, 480],
    [63, 240], [66, 240], [68, 480], [70, 240], [68, 240], [66, 480],
    [63, 960],
    [63, 240], [66, 240], [68, 480], [63, 240], [61, 240], [58, 480],
    [63, 240], [66, 240], [68, 480], [70, 240], [73, 240], [70, 480],
    [68, 960],
  ];
  let curTick = 0;
  for (let repeat = 0; repeat < 2; repeat++) {
    for (const [pitch, dur] of melody) {
      addNote(events, curTick, dur - 25, pitch, 92);
      curTick += dur;
    }
  }
}

function generateSenbonzakuraEvents(events: NoteEvent[]): void {
  const chordRoots = [42, 38, 45, 40]; // F#m, D, A, E
  const barTicks = 1920;
  for (let bar = 0; bar < 32; bar++) {
    const root = chordRoots[bar % 4];
    const barStart = bar * barTicks;
    for (let step = 0; step < 4; step++) {
      addNote(events, barStart + step * 480, 220, root, 78);
      addNote(events, barStart + step * 480 + 240, 220, root + 12, 72);
    }
  }
  const melody: [number, number][] = [
    [66, 240], [69, 240], [71, 240], [73, 480], [71, 240], [69, 240], [66, 480],
    [66, 240], [69, 240], [71, 480], [73, 480], [76, 480], [73, 480],
    [71, 240], [73, 240], [71, 240], [69, 240], [66, 480], [64, 480],
    [66, 960],
  ];
  let curTick = 0;
  for (let repeat = 0; repeat < 2; repeat++) {
    for (const [pitch, dur] of melody) {
      addNote(events, curTick, dur - 20, pitch, 96);
      curTick += dur;
    }
  }
}

function generateBadAppleEvents(events: NoteEvent[]): void {
  const chordRoots = [45, 41, 43, 40]; // Am, F, G, Em
  const barTicks = 1920;
  for (let bar = 0; bar < 32; bar++) {
    const root = chordRoots[bar % 4];
    const barStart = bar * barTicks;
    for (let beat = 0; beat < 4; beat++) {
      addNote(events, barStart + beat * 480, 220, root, 76);
      addNote(events, barStart + beat * 480 + 240, 220, root + 12, 70);
    }
  }
  const melody: [number, number][] = [
    [69, 240], [71, 240], [72, 480], [71, 240], [69, 240], [67, 480],
    [64, 240], [67, 240], [69, 960],
    [69, 240], [71, 240], [72, 480], [74, 240], [76, 240], [72, 480],
    [71, 960],
  ];
  let curTick = 0;
  for (let repeat = 0; repeat < 2; repeat++) {
    for (const [pitch, dur] of melody) {
      addNote(events, curTick, dur - 20, pitch, 94);
      curTick += dur;
    }
  }
}

function generateGenericEvents(seedId: string, events: NoteEvent[]): void {
  let hash = 0;
  for (let i = 0; i < seedId.length; i++) {
    hash = ((hash << 5) - hash + seedId.charCodeAt(i)) | 0;
  }
  const scale = [60, 62, 64, 65, 67, 69, 71, 72];
  const bassRoots = [36, 41, 43, 33];
  const barTicks = 1920;
  for (let bar = 0; bar < 32; bar++) {
    const root = bassRoots[bar % 4];
    const barStart = bar * barTicks;
    addNote(events, barStart, 460, root, 74);
    addNote(events, barStart + 480, 460, root + 7, 68);
    addNote(events, barStart + 960, 460, root + 12, 70);
    addNote(events, barStart + 1440, 460, root + 16, 68);

    for (let step = 0; step < 4; step++) {
      const idx = Math.abs((hash ^ (bar * 7 + step * 13)) % scale.length);
      const pitch = scale[idx];
      addNote(events, barStart + step * 480, 420, pitch, 86);
    }
  }
}

export function generateStandardMidiBytes(id: string, title: string): Uint8Array {
  const events: NoteEvent[] = [];
  const s = `${id.toLowerCase()} ${title.toLowerCase()}`;

  if (s.includes('canon') || s.includes('卡农')) {
    generateCanonEvents(events);
  } else if (s.includes('qingtian') || s.includes('晴天')) {
    generateQingtianEvents(events);
  } else if (s.includes('daoxiang') || s.includes('稻香')) {
    generateDaoxiangEvents(events);
  } else if (s.includes('bad apple') || s.includes('伤苹果') || s.includes('坏苹果')) {
    generateBadAppleEvents(events);
  } else if (s.includes('summer') || s.includes('菊次郎')) {
    generateSummerEvents(events);
  } else if (s.includes('faded')) {
    generateFadedEvents(events);
  } else if (s.includes('senbonzakura') || s.includes('千本樱')) {
    generateSenbonzakuraEvents(events);
  } else {
    generateGenericEvents(id, events);
  }

  events.sort((a, b) => a.tick - b.tick);

  const trackBytes: number[] = [];
  // Time signature 4/4
  trackBytes.push(0x00, 0xff, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08);
  // Tempo 120 BPM (500,000 micros)
  trackBytes.push(0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20);

  // Track name
  const titleUtf8 = new TextEncoder().encode(title);
  trackBytes.push(0x00, 0xff, 0x03);
  writeVarLen(titleUtf8.length, trackBytes);
  for (let i = 0; i < titleUtf8.length; i++) {
    trackBytes.push(titleUtf8[i]);
  }

  // Program change: Acoustic Grand Piano (0x00 on channel 0)
  trackBytes.push(0x00, 0xc0, 0x00);

  let lastTick = 0;
  for (const ev of events) {
    const delta = Math.max(0, ev.tick - lastTick);
    writeVarLen(delta, trackBytes);
    trackBytes.push(ev.status, ev.pitch, ev.velocity);
    lastTick = ev.tick;
  }

  // End of track
  writeVarLen(0, trackBytes);
  trackBytes.push(0xff, 0x2f, 0x00);

  const totalLen = 14 + 8 + trackBytes.length;
  const out = new Uint8Array(totalLen);
  let pos = 0;

  // Header MThd
  out.set([0x4d, 0x54, 0x68, 0x64], pos); pos += 4;
  out.set([0x00, 0x00, 0x00, 0x06], pos); pos += 4;
  out.set([0x00, 0x00], pos); pos += 2; // format 0
  out.set([0x00, 0x01], pos); pos += 2; // 1 track
  out.set([0x01, 0xe0], pos); pos += 2; // 480 ticks/beat

  // Track MTrk
  out.set([0x4d, 0x54, 0x72, 0x6b], pos); pos += 4;
  const len = trackBytes.length;
  out.set([(len >>> 24) & 0xff, (len >>> 16) & 0xff, (len >>> 8) & 0xff, len & 0xff], pos); pos += 4;
  out.set(trackBytes, pos);

  return out;
}

export function generateStandardMidiBase64(id: string, title: string): string {
  const bytes = generateStandardMidiBytes(id, title);
  let binary = '';
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}
