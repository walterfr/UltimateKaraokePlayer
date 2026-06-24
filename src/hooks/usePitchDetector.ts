import { useState, useEffect, useRef } from 'react';

// Auto-correlation algorithm to find fundamental frequency
function autoCorrelate(buf: Float32Array, sampleRate: number): number {
  let size = buf.length;
  let rms = 0;

  for (let i = 0; i < size; i++) {
    const val = buf[i];
    rms += val * val;
  }
  rms = Math.sqrt(rms / size);
  if (rms < 0.01) return -1; // Not enough signal

  let r1 = 0, r2 = size - 1, thres = 0.2;
  for (let i = 0; i < size / 2; i++)
    if (Math.abs(buf[i]) < thres) { r1 = i; break; }
  for (let i = 1; i < size / 2; i++)
    if (Math.abs(buf[size - i]) < thres) { r2 = size - i; break; }

  buf = buf.slice(r1, r2);
  size = buf.length;

  const c = new Array(size).fill(0);
  for (let i = 0; i < size; i++)
    for (let j = 0; j < size - i; j++)
      c[i] = c[i] + buf[j] * buf[j + i];

  let d = 0; 
  while (c[d] > c[d + 1]) d++;
  
  let maxval = -1, maxpos = -1;
  for (let i = d; i < size; i++) {
    if (c[i] > maxval) {
      maxval = c[i];
      maxpos = i;
    }
  }
  let T0 = maxpos;

  const x1 = c[T0 - 1], x2 = c[T0], x3 = c[T0 + 1];
  const a = (x1 + x3 - 2 * x2) / 2;
  const b = (x3 - x1) / 2;
  if (a) T0 = T0 - b / (2 * a);

  return sampleRate / T0;
}

export function usePitchDetector(deviceId?: string | null) {
  const [pitch, setPitch] = useState<number | null>(null);
  const audioCtxRef = useRef<AudioContext | null>(null);
  const analyserRef = useRef<AnalyserNode | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const rafRef = useRef<number>(0);

  useEffect(() => {
    if (!deviceId || deviceId === 'none') {
      setPitch(null);
      return;
    }

    let isMounted = true;

    async function start() {
      try {
        const stream = await navigator.mediaDevices.getUserMedia({
          audio: { deviceId: { exact: deviceId } }
        });
        
        if (!isMounted) {
          stream.getTracks().forEach(t => t.stop());
          return;
        }

        streamRef.current = stream;
        const ctx = new (window.AudioContext || (window as any).webkitAudioContext)();
        audioCtxRef.current = ctx;
        
        const analyser = ctx.createAnalyser();
        analyser.fftSize = 2048;
        analyserRef.current = analyser;

        const source = ctx.createMediaStreamSource(stream);
        source.connect(analyser);

        const buffer = new Float32Array(analyser.fftSize);

        const updatePitch = () => {
          if (!isMounted) return;
          analyser.getFloatTimeDomainData(buffer);
          const ac = autoCorrelate(buffer, ctx.sampleRate);
          if (ac !== -1 && ac !== Infinity && !isNaN(ac) && ac > 50 && ac < 2000) {
            // Convert Hz to MIDI Note (69 = A4 = 440Hz)
            const note = 12 * (Math.log(ac / 440) / Math.log(2)) + 69;
            setPitch(Math.round(note));
          } else {
            setPitch(null);
          }
          rafRef.current = requestAnimationFrame(updatePitch);
        };

        updatePitch();
      } catch (err) {
        console.error("Mic error:", err);
        setPitch(null);
      }
    }

    start();

    return () => {
      isMounted = false;
      cancelAnimationFrame(rafRef.current);
      if (streamRef.current) streamRef.current.getTracks().forEach(t => t.stop());
      if (audioCtxRef.current) audioCtxRef.current.close();
    };
  }, [deviceId]);

  return pitch;
}
