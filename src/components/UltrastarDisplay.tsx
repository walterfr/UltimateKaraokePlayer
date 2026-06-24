import React, { useState, useEffect, useMemo, useRef } from 'react';
import { usePitchDetector } from '../hooks/usePitchDetector';

export interface UltrastarNote {
  note_type: string;
  beat: number;
  length: number;
  pitch: number;
  text: string;
}

export interface UltrastarMetadata {
  title: string;
  artist: string;
  mp3: string;
  bpm: number;
  gap: number;
  video: string;
  cover: string;
  language: string;
  edition: string;
  genre: string;
  year: number;
  creator: string;
  notes: UltrastarNote[];
  total_duration: number;
}

interface UltrastarDisplayProps {
  metadata: UltrastarMetadata | null;
  isPlaying: boolean;
  currentTime: number;
  micDeviceId?: string;
}

interface Phrase {
  startBeat: number;
  endBeat: number;
  notes: UltrastarNote[];
}

const UltrastarDisplay: React.FC<UltrastarDisplayProps> = ({ metadata, isPlaying, currentTime, micDeviceId }) => {
  if (!metadata) {
    return (
      <div className="flex items-center justify-center h-full text-slate-500">
        <p className="text-lg italic">No Ultrastar file loaded</p>
      </div>
    );
  }

  const formatTime = (seconds: number): string => {
    const m = Math.floor(seconds / 60);
    const s = Math.floor(seconds % 60);
    return `${m}:${s.toString().padStart(2, '0')}`;
  };

  const beatDuration = metadata.bpm > 0 ? 60.0 / (metadata.bpm * 4.0) : 1.0;
  const gapSeconds = metadata.gap / 1000.0;
  const currentBeat = (currentTime - gapSeconds) / beatDuration;

  // 1. Group notes into phrases
  const phrases = useMemo(() => {
    const p: Phrase[] = [];
    let currentNotes: UltrastarNote[] = [];
    let startBeat = 0;
    
    metadata.notes.forEach(note => {
      if (note.note_type === '-' || note.note_type === 'E') {
        if (currentNotes.length > 0) {
          const endBeat = currentNotes[currentNotes.length - 1].beat + currentNotes[currentNotes.length - 1].length;
          p.push({ startBeat, endBeat, notes: currentNotes });
          currentNotes = [];
        }
        startBeat = note.beat;
      } else {
        if (currentNotes.length === 0) startBeat = note.beat;
        currentNotes.push(note);
      }
    });
    
    if (currentNotes.length > 0) {
      const endBeat = currentNotes[currentNotes.length - 1].beat + currentNotes[currentNotes.length - 1].length;
      p.push({ startBeat, endBeat, notes: currentNotes });
    }
    return p;
  }, [metadata]);

  // 2. Find active phrase
  const activePhraseIdx = useMemo(() => {
    for (let i = 0; i < phrases.length; i++) {
      const nextStart = phrases[i+1]?.startBeat ?? Infinity;
      // We are in phrase i if we are past its startBeat (or just before it) and before the next phrase
      if (currentBeat < nextStart) {
        return i;
      }
    }
    return phrases.length - 1;
  }, [currentBeat, phrases]);

  const currentPhrase = phrases[activePhraseIdx];
  const nextPhrase = phrases[activePhraseIdx + 1];

  // 3. Pitch Detection
  const detectedPitch = usePitchDetector(micDeviceId);
  const [pitchHistory, setPitchHistory] = useState<{beat: number, pitch: number}[]>([]);
  const lastTimeRef = useRef(currentTime);

  useEffect(() => {
    // Clear history when we change phrase or seek backward
    if (currentTime < lastTimeRef.current || pitchHistory.length > 300) {
      setPitchHistory([]);
    }
    if (detectedPitch !== null) {
      setPitchHistory(prev => [...prev, { beat: currentBeat, pitch: detectedPitch }]);
    }
    lastTimeRef.current = currentTime;
  }, [detectedPitch, currentBeat, currentTime]);

  // 3. Pitch calculations for the active phrase to normalize Y axis
  const minPitch = currentPhrase ? currentPhrase.notes.reduce((m, n) => Math.min(m, n.pitch), 100) : 0;
  const maxPitch = currentPhrase ? currentPhrase.notes.reduce((m, n) => Math.max(m, n.pitch), -100) : 1;
  const pitchRange = Math.max(maxPitch - minPitch, 12); // Minimum visual range of 1 octave (12 semitones)

  const noteColors: Record<string, string> = {
    ':': 'bg-blue-500',
    '*': 'bg-yellow-400',
    'F': 'bg-purple-500',
    'R': 'bg-green-500',
    'r': 'bg-green-300',
    'G': 'bg-pink-500',
  };

  const renderPhrasePianoRoll = (phrase: Phrase, isActivePhrase: boolean) => {
    const leadInBeats = 8; // Show 8 beats before the phrase starts
    const viewStartBeat = phrase.startBeat - leadInBeats;
    const totalBeatsView = Math.max(phrase.endBeat - viewStartBeat, 10); 
    
    // Calculate sweeper position
    const sweeperPercent = ((currentBeat - viewStartBeat) / totalBeatsView) * 90;
    const isSweeperVisible = isActivePhrase && sweeperPercent >= 0 && currentBeat <= phrase.endBeat;

    return (
      <div className={`relative w-full h-full transition-opacity duration-500 ${isActivePhrase ? 'opacity-100' : 'opacity-20'}`}>
        {phrase.notes.map((note, idx) => {
          const isNoteActive = currentBeat >= note.beat && currentBeat < (note.beat + note.length);
          const isNotePast = currentBeat >= (note.beat + note.length);
          
          // X-Axis mapping with lead-in
          const leftPercent = ((note.beat - viewStartBeat) / totalBeatsView) * 90; 
          const widthPercent = (note.length / totalBeatsView) * 90;
          
          // Y-Axis mapping (higher pitch = lower top value)
          const pitchOffset = note.pitch - minPitch;
          const topPercent = 100 - (((pitchOffset + 2) / (pitchRange + 4)) * 100); 

          const colorClass = noteColors[note.note_type] || 'bg-slate-500';

          return (
            <div
              key={idx}
              className={`absolute rounded-full transition-all duration-100 flex items-center justify-center overflow-hidden
                ${isNoteActive ? 'scale-110 shadow-lg shadow-white/50 z-10' : 'z-0'}
                ${isNotePast ? 'opacity-50 grayscale' : 'opacity-90'}
                ${note.note_type === '*' ? 'animate-pulse' : ''}
              `}
              style={{
                left: `${leftPercent}%`,
                width: `${Math.max(widthPercent, 2)}%`,
                top: `${topPercent}%`,
                height: '16px',
                marginTop: '-8px',
                backgroundColor: isNoteActive ? '#3b82f6' : undefined, 
              }}
            >
              <div className={`w-full h-full ${colorClass}`} />
            </div>
          );
        })}

        {/* Progress Sweeper */}
        {isSweeperVisible && (
          <div 
            className="absolute top-0 bottom-0 w-1 bg-white shadow-[0_0_15px_white] z-20"
            style={{ left: `${sweeperPercent}%` }}
          />
        )}

        {/* Pitch History (Octave-agnostic matching to closest note) */}
        {isActivePhrase && pitchHistory.map((pt, i) => {
          // Only show points within this phrase's view
          if (pt.beat < viewStartBeat || pt.beat > phrase.endBeat) return null;

          // Find the active target note at this beat to normalize octave
          let targetPitch = minPitch + (pitchRange / 2); // default center
          const activeNote = phrase.notes.find(n => pt.beat >= n.beat && pt.beat < n.beat + n.length);
          if (activeNote) {
            targetPitch = activeNote.pitch;
          }

          // Octave normalization: force sung pitch to the closest octave of target pitch
          const noteClassTarget = (targetPitch % 12 + 12) % 12;
          const noteClassSung = (pt.pitch % 12 + 12) % 12;
          const classDiff = noteClassSung - noteClassTarget;
          
          let normalizedPitch = targetPitch;
          if (classDiff > 6) normalizedPitch = targetPitch + classDiff - 12;
          else if (classDiff < -6) normalizedPitch = targetPitch + classDiff + 12;
          else normalizedPitch = targetPitch + classDiff;

          const ptLeftPercent = ((pt.beat - viewStartBeat) / totalBeatsView) * 90;
          const pitchOffset = normalizedPitch - minPitch;
          const ptTopPercent = 100 - (((pitchOffset + 2) / (pitchRange + 4)) * 100);

          return (
            <div 
              key={i}
              className="absolute w-2 h-2 rounded-full bg-cyan-300 shadow-[0_0_8px_cyan] z-30"
              style={{
                left: `calc(${ptLeftPercent}% - 4px)`,
                top: `calc(${ptTopPercent}% - 4px)`,
              }}
            />
          );
        })}
        
        {/* Approaching text if waiting for gap/lead-in */}
        {isActivePhrase && currentBeat < phrase.startBeat && currentBeat >= viewStartBeat && (
          <div 
            className="absolute top-1/2 -translate-y-1/2 text-white/50 font-bold italic text-xl z-0"
            style={{ left: `${((phrase.startBeat - viewStartBeat) / totalBeatsView) * 90 + 2}%` }}
          >
            Get ready...
          </div>
        )}
      </div>
    );
  };

  const renderLyrics = (phrase: Phrase | undefined) => {
    if (!phrase) return null;
    return (
      <div className="flex flex-wrap justify-center gap-x-1 gap-y-2 text-2xl md:text-4xl font-bold px-4">
        {phrase.notes.map((note, idx) => {
          const isNoteActive = currentBeat >= note.beat && currentBeat < (note.beat + note.length);
          const isNotePast = currentBeat >= (note.beat + note.length);
          
          let colorClass = 'text-white/40';
          if (isNoteActive) colorClass = 'text-blue-400 drop-shadow-[0_0_8px_rgba(59,130,246,0.8)] scale-110';
          else if (isNotePast) colorClass = 'text-white drop-shadow-md';

          // Ensure spaces are respected if they exist at the start/end of the syllable
          const text = note.text.replace(/ /g, '\u00A0');

          return (
            <span key={idx} className={`transition-all duration-100 ${colorClass}`}>
              {text}
            </span>
          );
        })}
      </div>
    );
  };

  return (
    <div className="flex flex-col h-full w-full bg-slate-950">
      <div className="text-center px-4 py-3 bg-slate-900 border-b border-slate-800 shrink-0">
        <h2 className="text-2xl font-bold text-white truncate drop-shadow-md">
          {metadata.title || 'Unknown Title'}
        </h2>
        {metadata.artist && (
          <p className="text-md text-blue-400 truncate">{metadata.artist}</p>
        )}
        <div className="flex items-center justify-center gap-4 text-xs text-slate-500 mt-2">
          <span>{metadata.bpm > 0 ? `${metadata.bpm} BPM` : 'No BPM'}</span>
          <span>{formatTime(currentTime)} / {formatTime(metadata.total_duration)}</span>
        </div>
      </div>

      <div className="flex-1 relative overflow-hidden flex flex-col p-6">
        {/* Piano Roll Area */}
        <div className="flex-1 relative bg-slate-900/50 rounded-xl border border-slate-800/50 mb-6 overflow-hidden">
          {/* Horizontal lines for visual structure */}
          <div className="absolute inset-0 flex flex-col justify-between opacity-10 pointer-events-none">
            {[...Array(5)].map((_, i) => (
              <div key={i} className="w-full h-px bg-white" />
            ))}
          </div>

          {currentPhrase && renderPhrasePianoRoll(currentPhrase, true)}
        </div>

        {/* Lyrics Area */}
        <div className="shrink-0 min-h-[120px] flex flex-col justify-center items-center gap-4 bg-slate-900/30 rounded-xl p-4 border border-slate-800/30">
          {renderLyrics(currentPhrase)}
          {nextPhrase && currentBeat > currentPhrase.endBeat - 2 && (
            <div className="text-sm text-slate-500 italic mt-2 animate-pulse">
              Up next: {nextPhrase.notes.map(n => n.text).join('').trim()}
            </div>
          )}
        </div>
      </div>

      <div className="h-1.5 bg-slate-800 shrink-0">
        <div
          className="h-full bg-gradient-to-r from-blue-600 to-blue-400 transition-all duration-200"
          style={{ width: `${(currentTime / metadata.total_duration) * 100}%` }}
        />
      </div>
    </div>
  );
};

export default UltrastarDisplay;
