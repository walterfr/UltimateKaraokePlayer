import React, { useEffect, useRef } from 'react';
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

interface CdgCanvasProps {}

const CDG_WIDTH = 300;
const CDG_HEIGHT = 216;

const CdgCanvas: React.FC<CdgCanvasProps> = () => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  
  // State refs to persist between renders without causing React re-renders
  const colorIndexBufferRef = useRef<Uint8Array>(new Uint8Array(CDG_WIDTH * CDG_HEIGHT));
  // Palette in ABGR format (for direct Uint32Array assignment into ImageData)
  const paletteRef = useRef<Uint32Array>(new Uint32Array(16));
  
  useEffect(() => {
    // Inicializa a paleta padrão (preto) com alpha total
    for(let i=0; i<16; i++) {
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

    // Converte de 12-bit RGB para 32-bit ABGR
    const setPaletteColor = (index: number, rgb12: number) => {
      const r = (rgb12 >> 8) & 0x0F;
      const g = (rgb12 >> 4) & 0x0F;
      const b = rgb12 & 0x0F;
      
      const r8 = r * 17;
      const g8 = g * 17;
      const b8 = b * 17;
      
      // A=0xFF (255) | B | G | R
      palette[index] = 0xFF000000 | (b8 << 16) | (g8 << 8) | r8;
    };

    let unlistenFn: () => void;

    // Escuta batches do backend via Tauri Events
    listen<{ commands: CdgCommand[] }>('cdg_batch', (event) => {
      let needsRedraw = false;
      const commands = event.payload.commands;

      commands.forEach(cmd => {
        switch (cmd.type) {
          case 'MemoryPreset': {
            const color = cmd.data.color;
            indexBuffer.fill(color);
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
          case 'LoadColorTableLow': {
            cmd.data.colors.forEach((c, i) => setPaletteColor(i, c));
            needsRedraw = true; // Força re-draw na mudança de paleta
            break;
          }
          case 'LoadColorTableHigh': {
            cmd.data.colors.forEach((c, i) => setPaletteColor(i + 8, c));
            needsRedraw = true;
            break;
          }
          case 'TileBlockNormal':
          case 'TileBlockXor': {
            const { color0, color1, row, col, pixels } = cmd.data;
            const startX = col * 6;
            const startY = row * 12;
            
            if (startX >= CDG_WIDTH || startY >= CDG_HEIGHT) break;
            
            const isXor = cmd.type === 'TileBlockXor';

            for (let py = 0; py < 12; py++) {
              const pixelRow = pixels[py];
              const actualY = startY + py;
              if (actualY >= CDG_HEIGHT) continue;
              
              for (let px = 0; px < 6; px++) {
                const actualX = startX + px;
                if (actualX >= CDG_WIDTH) continue;
                
                const bit = (pixelRow >> (5 - px)) & 0x01;
                const colorIndex = bit ? color1 : color0;
                
                const idx = actualY * CDG_WIDTH + actualX;
                if (isXor) {
                  indexBuffer[idx] ^= colorIndex;
                } else {
                  indexBuffer[idx] = colorIndex;
                }
              }
            }
            needsRedraw = true;
            break;
          }
        }
      });

      if (needsRedraw) {
        const imageData = ctx.createImageData(CDG_WIDTH, CDG_HEIGHT);
        const data32 = new Uint32Array(imageData.data.buffer);
        
        for (let i = 0; i < indexBuffer.length; i++) {
          data32[i] = palette[indexBuffer[i]];
        }
        
        ctx.putImageData(imageData, 0, 0);
      }
    }).then(f => unlistenFn = f);

    return () => {
      if (unlistenFn) unlistenFn();
    };
  }, []);

  return (
    <div className="flex flex-col items-center justify-center w-full h-full bg-black overflow-hidden relative">
      <div className="relative aspect-[300/216] max-h-full max-w-full w-[800px]">
        <canvas 
          ref={canvasRef} 
          width={CDG_WIDTH} 
          height={CDG_HEIGHT} 
          className="absolute inset-0 w-full h-full object-contain shadow-[0_0_80px_rgba(0,100,255,0.15)] border-4 border-[#12121a] rounded-lg"
          style={{ imageRendering: 'pixelated' }}
        />
        <div className="absolute inset-0 pointer-events-none bg-[url('data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAACCAYAAACZgbYnAAAAEElEQVQImWNgYGD4z8DAAAAAYAAY09L6LwAAAABJRU5ErkJggg==')] opacity-20 mix-blend-overlay rounded-lg"></div>
      </div>
    </div>
  );
};

export default CdgCanvas;
