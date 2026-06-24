import React, { useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';

export type CdgCommand = 
  | { type: 'MemoryPreset', data: { color: number, repeat: number } }
  | { type: 'BorderPreset', data: { color: number } }
  | { type: 'TileBlockNormal', data: { color0: number, color1: number, row: number, col: number, pixels: number[] } }
  | { type: 'ScrollPreset', data: { color: number, h_cmd: number, h_offset: number, v_cmd: number, v_offset: number } }
  | { type: 'ScrollCopy', data: { h_cmd: number, h_offset: number, v_cmd: number, v_offset: number } }
  | { type: 'DefineTransparentColor', data: { color: number } }
  | { type: 'LoadColorTableLow', data: { colors: number[] } }
  | { type: 'LoadColorTableHigh', data: { colors: number[] } }
  | { type: 'TileBlockXor', data: { color0: number, color1: number, row: number, col: number, pixels: number[] } };

const CDG_WIDTH = 300;
const CDG_HEIGHT = 216;

const CdgCanvas: React.FC = () => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const colorIndexBufferRef = useRef<Uint8Array>(new Uint8Array(CDG_WIDTH * CDG_HEIGHT));
  const paletteRef = useRef<Uint32Array>(new Uint32Array(16));
  const [progress, setProgress] = useState({ current: 0, total: 0 });

  useEffect(() => {
    for (let i = 0; i < 16; i++) {
      if (paletteRef.current[i] === 0) {
        paletteRef.current[i] = 0xFF000000;
      }
    }
  }, []);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d', { alpha: false });
    if (!ctx) return;

    const indexBuffer = colorIndexBufferRef.current;
    const palette = paletteRef.current;

    const setPaletteColor = (index: number, rgb12: number) => {
      const r = (rgb12 >> 8) & 0x0F;
      const g = (rgb12 >> 4) & 0x0F;
      const b = rgb12 & 0x0F;
      palette[index] = 0xFF000000 | ((b * 17) << 16) | ((g * 17) << 8) | (r * 17);
    };

    const render = () => {
      const imageData = ctx.createImageData(CDG_WIDTH, CDG_HEIGHT);
      const data32 = new Uint32Array(imageData.data.buffer);
      for (let i = 0; i < indexBuffer.length; i++) {
        data32[i] = palette[indexBuffer[i]];
      }
      ctx.putImageData(imageData, 0, 0);
    };

    let unlisten: (() => void) | undefined;
    let isCancelled = false;

    listen<{ commands: CdgCommand[]; current_time: number; total_duration: number }>('cdg_batch', (event) => {
      let needsRedraw = false;
      const types = event.payload.commands.map(c => c.type);
      if (types.length > 0) console.log('[CDG] batch:', types);
      setProgress({ current: event.payload.current_time, total: event.payload.total_duration });

      for (const cmd of event.payload.commands) {
        switch (cmd.type) {
          case 'MemoryPreset': {
            indexBuffer.fill(cmd.data.color);
            needsRedraw = true;
            break;
          }
          case 'BorderPreset': {
            const color = cmd.data.color;
            for (let y = 0; y < CDG_HEIGHT; y++) {
              for (let x = 0; x < CDG_WIDTH; x++) {
                if (x < 6 || x >= CDG_WIDTH - 6 || y < 12 || y >= CDG_HEIGHT - 12) {
                  indexBuffer[y * CDG_WIDTH + x] = color;
                }
              }
            }
            needsRedraw = true;
            break;
          }
          case 'ScrollPreset':
          case 'ScrollCopy': {
            const { h_cmd, v_cmd } = cmd.data;
            let h_shift = 0;
            if (h_cmd === 1) h_shift = 6;
            else if (h_cmd === 2) h_shift = -6;

            let v_shift = 0;
            if (v_cmd === 1) v_shift = 12;
            else if (v_cmd === 2) v_shift = -12;

            if (h_shift !== 0 || v_shift !== 0) {
              const newBuffer = new Uint8Array(CDG_WIDTH * CDG_HEIGHT);
              const isCopy = cmd.type === 'ScrollCopy';
              const bgColor = cmd.type === 'ScrollPreset' ? cmd.data.color : 0;

              for (let y = 0; y < CDG_HEIGHT; y++) {
                for (let x = 0; x < CDG_WIDTH; x++) {
                  let srcX = x - h_shift;
                  let srcY = y - v_shift;
                  
                  if (isCopy) {
                    if (srcX < 0) srcX += CDG_WIDTH;
                    else if (srcX >= CDG_WIDTH) srcX -= CDG_WIDTH;
                    if (srcY < 0) srcY += CDG_HEIGHT;
                    else if (srcY >= CDG_HEIGHT) srcY -= CDG_HEIGHT;
                    newBuffer[y * CDG_WIDTH + x] = indexBuffer[srcY * CDG_WIDTH + srcX];
                  } else {
                    if (srcX < 0 || srcX >= CDG_WIDTH || srcY < 0 || srcY >= CDG_HEIGHT) {
                      newBuffer[y * CDG_WIDTH + x] = bgColor;
                    } else {
                      newBuffer[y * CDG_WIDTH + x] = indexBuffer[srcY * CDG_WIDTH + srcX];
                    }
                  }
                }
              }
              indexBuffer.set(newBuffer);
              needsRedraw = true;
            }
            break;
          }
          case 'LoadColorTableLow':
            cmd.data.colors.forEach((c, i) => setPaletteColor(i, c));
            needsRedraw = true;
            break;
          case 'LoadColorTableHigh':
            cmd.data.colors.forEach((c, i) => setPaletteColor(i + 8, c));
            needsRedraw = true;
            break;
          case 'TileBlockNormal':
          case 'TileBlockXor': {
            const { color0, color1, row, col, pixels } = cmd.data;
            const startX = col * 6;
            const startY = row * 12;
            if (startX >= CDG_WIDTH || startY >= CDG_HEIGHT) break;
            const isXor = cmd.type === 'TileBlockXor';
            for (let py = 0; py < 12; py++) {
              const actualY = startY + py;
              if (actualY >= CDG_HEIGHT) continue;
              const pixelRow = pixels[py];
              for (let px = 0; px < 6; px++) {
                const actualX = startX + px;
                if (actualX >= CDG_WIDTH) continue;
                const bit = (pixelRow >> (5 - px)) & 1;
                const ci = bit ? color1 : color0;
                const idx = actualY * CDG_WIDTH + actualX;
                indexBuffer[idx] = isXor ? indexBuffer[idx] ^ ci : ci;
              }
            }
            needsRedraw = true;
            break;
          }
        }
      }

      if (needsRedraw) render();
    }).then((f) => {
      if (isCancelled) f();
      else unlisten = f;
    });

    return () => {
      isCancelled = true;
      if (unlisten) unlisten();
    };
  }, []);

  const fmt = (s: number) => `${Math.floor(s / 60)}:${Math.floor(s % 60).toString().padStart(2, '0')}`;

  return (
    <div className="flex flex-col items-center justify-center w-full h-full bg-black overflow-hidden relative">
      <div className="relative w-full max-w-[800px]" style={{ aspectRatio: '300/216' }}>
        <canvas
          ref={canvasRef}
          width={CDG_WIDTH}
          height={CDG_HEIGHT}
          className="absolute inset-0 w-full h-full border-2 border-slate-800 rounded-lg"
          style={{ imageRendering: 'pixelated' }}
        />
      </div>
      {progress.total > 0 && (
        <div className="w-full max-w-[800px] px-4 mt-2">
          <div className="h-1.5 bg-slate-800 rounded-full overflow-hidden">
            <div
              className="h-full bg-blue-500 transition-all duration-200 rounded-full"
              style={{ width: `${(progress.current / progress.total) * 100}%` }}
            />
          </div>
          <div className="flex justify-between text-[10px] text-slate-500 mt-0.5">
            <span>{fmt(progress.current)}</span>
            <span>{fmt(progress.total)}</span>
          </div>
        </div>
      )}
    </div>
  );
};

export default CdgCanvas;
