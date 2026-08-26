import { ref, onUnmounted } from 'vue';

interface MidiDevice {
  id: string;
  name: string;
  kind: 'input' | 'output';
}

let midiAccess: MIDIAccess | null = null;
let midiOutput: MIDIOutput | null = null;
let midiInput: MIDIInput | null = null;
let midiInputHandler: ((status: number, data: number[], timestamp: number) => void) | null = null;

export const webMidiDevices = ref<MidiDevice[]>([]);
export const webMidiConnected = ref(false);
export const webMidiDeviceName = ref<string | null>(null);
export const webMidiLoading = ref(false);

async function getMidiAccess(): Promise<MIDIAccess> {
  if (!midiAccess) {
    midiAccess = await navigator.requestMIDIAccess({ sysex: false });
  }
  return midiAccess;
}

export async function listWebMidiDevices(): Promise<MidiDevice[]> {
  try {
    const access = await getMidiAccess();
    const devices: MidiDevice[] = [];
    for (const [id, output] of access.outputs) {
      devices.push({ id: `output:${id}`, name: output.name || id, kind: 'output' });
    }
    for (const [id, input] of access.inputs) {
      devices.push({ id: `input:${id}`, name: input.name || id, kind: 'input' });
    }
    webMidiDevices.value = devices;
    return devices;
  } catch (e: any) {
    console.warn('[Web MIDI] Failed to list devices:', e.message);
    return [];
  }
}

export async function connectWebMidiDevice(deviceId: string): Promise<{ connected: boolean; device_name?: string }> {
  webMidiLoading.value = true;
  try {
    const access = await getMidiAccess();
    const isOutput = deviceId.startsWith('output:');
    const realId = deviceId.replace(/^(output|input):/, '');

    if (isOutput) {
      if (midiOutput) {
        try { midiOutput.close(); } catch { /* ignore */ }
        midiOutput = null;
      }
      const output = access.outputs.get(realId);
      if (!output) {
        return { connected: false };
      }
      midiOutput = output;
      webMidiConnected.value = true;
      webMidiDeviceName.value = output.name || realId;
      return { connected: true, device_name: output.name || realId };
    } else {
      if (midiInput) {
        try { midiInput.onmidimessage = null; } catch { /* ignore */ }
        midiInput = null;
      }
      const input = access.inputs.get(realId);
      if (!input) {
        return { connected: false };
      }
      midiInput = input;
      webMidiConnected.value = true;
      webMidiDeviceName.value = input.name || realId;
      return { connected: true, device_name: input.name || realId };
    }
  } catch (e: any) {
    webMidiConnected.value = false;
    webMidiDeviceName.value = null;
    return { connected: false };
  } finally {
    webMidiLoading.value = false;
  }
}

export async function disconnectWebMidi(): Promise<void> {
  if (midiOutput) {
    try { midiOutput.close(); } catch { /* ignore */ }
    midiOutput = null;
  }
  if (midiInput) {
    try { midiInput.onmidimessage = null; } catch { /* ignore */ }
    midiInput = null;
  }
  webMidiConnected.value = false;
  webMidiDeviceName.value = null;
}

export function onWebMidiMessage(handler: (status: number, data: number[], timestamp: number) => void): () => void {
  midiInputHandler = handler;
  return () => {
    midiInputHandler = null;
  };
}

export function sendWebMidiNoteOn(note: number, velocity: number = 100, channel: number = 0): void {
  if (!midiOutput) return;
  const status = 0x90 | channel;
  midiOutput.send([status, note, velocity]);
}

export function sendWebMidiNoteOff(note: number, channel: number = 0): void {
  if (!midiOutput) return;
  const status = 0x80 | channel;
  midiOutput.send([status, note, 0]);
}

export function sendWebMidiProgramChange(program: number, channel: number = 0): void {
  if (!midiOutput) return;
  const status = 0xC0 | channel;
  midiOutput.send([status, program]);
}

export function sendWebMidiControlChange(controller: number, value: number, channel: number = 0): void {
  if (!midiOutput) return;
  const status = 0xB0 | channel;
  midiOutput.send([status, controller, value]);
}

export function isWebMidiSupported(): boolean {
  return typeof navigator !== 'undefined' && 'requestMIDIAccess' in navigator;
}

if (typeof navigator !== 'undefined' && 'requestMIDIAccess' in navigator) {
  navigator.requestMIDIAccess({ sysex: false }).then((access) => {
    access.onstatechange = () => {
      void listWebMidiDevices();
    };
  }).catch(() => {
    // Web MIDI not available
  });
}

onUnmounted(() => {
  disconnectWebMidi();
});
