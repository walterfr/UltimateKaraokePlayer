import React, { useState, useEffect, useRef } from 'react';

export interface MidiSyllable {
  text: string;
  start_seconds: number;
}

export interface MidiLyric {
  syllables: MidiSyllable[];
  start_seconds: number;
  end_seconds: number;
  text: string;
  start_tick: number;
  end_tick: number;
  duration_seconds: number;
}

export interface SynthTrackInfo {
  name: string;
  note_count: number;
}

export interface SynthMetadata {
  title: string;
  artist: string;
  format: number;
  tracks: SynthTrackInfo[];
  total_ticks: number;
  total_seconds: number;
  lyrics: MidiLyric[];
}

interface SynthDisplayProps {
  metadata: SynthMetadata | null;
  isPlaying: boolean;
  currentTime: number;
}

const SynthDisplay: React.FC<SynthDisplayProps> = ({ metadata, isPlaying, currentTime }) => {
  const [currentLineIdx, setCurrentLineIdx] = useState(-1);
  const [nextLineIdx, setNextLineIdx] = useState(-1);
  const prevLineRef = useRef<HTMLDivElement>(null);
  const curLineRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!metadata || metadata.lyrics.length === 0) return;

    let cur = -1;
    let next = -1;
    for (let i = 0; i < metadata.lyrics.length; i++) {
      const l = metadata.lyrics[i];
      if (currentTime >= l.start_seconds && currentTime < l.end_seconds) {
        cur = i;
        next = i + 1 < metadata.lyrics.length ? i + 1 : -1;
        break;
      }
      // Mostrar próxima linha antes de ela começar (2s de antecipação)
      if (cur === -1 && l.start_seconds > currentTime && l.start_seconds - currentTime <= 2.0) {
        next = i;
      }
    }
    setCurrentLineIdx(cur);
    setNextLineIdx(cur >= 0 ? (cur + 1 < metadata.lyrics.length ? cur + 1 : -1) : next);

    // Scroll suave para a linha atual
    if (cur >= 0 && curLineRef.current) {
      curLineRef.current.scrollIntoView({ behavior: 'smooth', block: 'center' });
    }
  }, [currentTime, metadata]);

  if (!metadata) {
    return (
      <div className="flex items-center justify-center h-full text-slate-500">
        <p className="text-lg italic">Nenhum arquivo MIDI/KAR carregado</p>
      </div>
    );
  }

  const fmt = (s: number) => `${Math.floor(s / 60)}:${Math.floor(s % 60).toString().padStart(2, '0')}`;

  const currentLine = currentLineIdx >= 0 ? metadata.lyrics[currentLineIdx] : null;
  const nextLine = nextLineIdx >= 0 ? metadata.lyrics[nextLineIdx] : null;
  const prevLine = currentLineIdx > 0 ? metadata.lyrics[currentLineIdx - 1] : null;

  return (
    <div className="flex flex-col h-full w-full bg-black">
      {/* Cabeçalho */}
      <div className="text-center px-4 py-2 bg-slate-900/70 border-b border-slate-800 shrink-0">
        <h2 className="text-lg font-bold text-white truncate">{metadata.title || 'MIDI/KAR'}</h2>
        {metadata.artist && <p className="text-xs text-blue-400 truncate">{metadata.artist}</p>}
        <div className="text-[10px] text-slate-600 mt-0.5">{fmt(currentTime)} / {fmt(metadata.total_seconds)}</div>
      </div>

      {/* Área de letras — mostra 3 linhas: anterior, atual, próxima */}
      <div className="flex-1 flex flex-col items-center justify-center gap-6 px-6 overflow-hidden">

        {/* Linha anterior (esmaecida) */}
        <div ref={prevLineRef} className="text-center text-slate-600 text-xl font-semibold transition-all duration-500 min-h-[2rem]">
          {prevLine?.text || ''}
        </div>

        {/* Linha ATUAL com sílabas destacadas progressivamente */}
        <div ref={curLineRef} className="text-center text-3xl font-bold leading-snug transition-all duration-300 min-h-[3rem]">
          {currentLine ? (
            currentLine.syllables.map((syl, i) => {
              const isSung = currentTime >= syl.start_seconds;
              const isActive = isSung && (
                i + 1 >= currentLine.syllables.length ||
                currentTime < currentLine.syllables[i + 1].start_seconds
              );
              return (
                <span
                  key={i}
                  className={`transition-colors duration-150 ${
                    isActive
                      ? 'text-yellow-300 drop-shadow-[0_0_8px_rgba(253,224,71,0.8)]'
                      : isSung
                        ? 'text-white'
                        : 'text-slate-400'
                  }`}
                >
                  {syl.text}
                </span>
              );
            })
          ) : (
            <span className="text-slate-700 text-lg italic">
              {metadata.lyrics.length === 0 ? 'Sem letras neste arquivo' : '♪'}
            </span>
          )}
        </div>

        {/* Próxima linha (pré-visualização) */}
        <div className="text-center text-slate-500 text-xl font-semibold transition-all duration-500 min-h-[2rem]">
          {nextLine?.text || ''}
        </div>
      </div>

      {/* Barra de progresso */}
      <div className="h-1.5 bg-slate-800 shrink-0">
        <div
          className="h-full bg-gradient-to-r from-orange-500 to-yellow-400 transition-all duration-200"
          style={{ width: `${Math.min((currentTime / metadata.total_seconds) * 100, 100)}%` }}
        />
      </div>
    </div>
  );
};

export default SynthDisplay;
