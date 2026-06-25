import React, { useState, useEffect, useMemo, useRef } from 'react';
import { usePitchDetector } from '../hooks/usePitchDetector';
import { convertFileSrc } from '@tauri-apps/api/tauri';

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
  songFilePath?: string;
  onFinish?: () => void;
}

interface Phrase {
  startBeat: number;
  endBeat: number;
  notes: UltrastarNote[];
}

const UltrastarDisplay: React.FC<UltrastarDisplayProps> = ({ metadata, isPlaying, currentTime, micDeviceId, songFilePath, onFinish }) => {
  if (!metadata) {
    return (
      <div className="flex items-center justify-center h-full text-slate-500">
        <p className="text-lg italic">No Ultrastar file loaded</p>
      </div>
    );
  }

  const beatDuration = metadata.bpm > 0 ? 60.0 / (metadata.bpm * 4.0) : 1.0;
  const gapSeconds = metadata.gap / 1000.0;
  const currentBeat = (currentTime - gapSeconds) / beatDuration;

  const [mediaUrl, setMediaUrl] = useState<{ type: 'video' | 'image', url: string } | null>(null);

  useEffect(() => {
    if (metadata && songFilePath) {
      const dir = songFilePath.replace(/\\/g, '/').substring(0, songFilePath.replace(/\\/g, '/').lastIndexOf('/'));
      if (metadata.video) {
        setMediaUrl({ type: 'video', url: convertFileSrc(`${dir}/${metadata.video}`) });
      } else if (metadata.cover) {
        setMediaUrl({ type: 'image', url: convertFileSrc(`${dir}/${metadata.cover}`) });
      } else {
        setMediaUrl(null);
      }
    }
  }, [metadata, songFilePath]);

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
      if (currentBeat < nextStart) return i;
    }
    return phrases.length - 1;
  }, [currentBeat, phrases]);

  const currentPhrase = phrases[activePhraseIdx];
  const nextPhrase = phrases[activePhraseIdx + 1];

  // 3. Scoring System
  const totalScoreableBeats = useMemo(() => {
    let sum = 0;
    metadata.notes.forEach(n => {
      if (n.note_type === ':' || n.note_type === 'F') sum += n.length;
      else if (n.note_type === '*') sum += n.length * 2;
    });
    return Math.max(sum, 1);
  }, [metadata]);

  const detectedPitch = usePitchDetector(micDeviceId);
  const [pitchHistory, setPitchHistory] = useState<{beat: number, pitch: number, isHit: boolean}[]>([]);
  const [score, setScore] = useState(0);
  
  const lastTimeRef = useRef(currentTime);
  const lastBeatRef = useRef(currentBeat);

  useEffect(() => {
    // Detect seeking/restarting
    if (currentTime < lastTimeRef.current || currentBeat < lastBeatRef.current) {
      setScore(0);
      setPitchHistory([]);
    }

    const deltaBeat = Math.max(0, currentBeat - lastBeatRef.current);
    let isHit = false;
    
    if (detectedPitch !== null && deltaBeat > 0 && currentPhrase) {
      const activeNote = currentPhrase.notes.find(n => currentBeat >= n.beat && currentBeat < n.beat + n.length);
      if (activeNote && activeNote.note_type !== '-' && activeNote.note_type !== 'E') {
        const targetClass = (activeNote.pitch % 12 + 12) % 12;
        const sungClass = (detectedPitch % 12 + 12) % 12;
        const diff = Math.abs(targetClass - sungClass);
        
        // 1 semitone tolerance
        if (diff <= 1 || diff >= 11) {
          isHit = true;
          const multiplier = activeNote.note_type === '*' ? 2 : 1;
          const pointsToAdd = (10000 / totalScoreableBeats) * deltaBeat * multiplier;
          setScore(s => Math.min(10000, s + pointsToAdd));
        }
      }
    }

    if (detectedPitch !== null) {
      // Limit history size
      setPitchHistory(prev => {
        const next = [...prev, { beat: currentBeat, pitch: detectedPitch, isHit }];
        if (next.length > 200) return next.slice(-200);
        return next;
      });
    }
    
    lastTimeRef.current = currentTime;
    lastBeatRef.current = currentBeat;
  }, [detectedPitch, currentBeat, currentTime, currentPhrase, totalScoreableBeats]);

  // Pitch calculations for the active phrase to normalize Y axis
  const minPitch = currentPhrase ? currentPhrase.notes.reduce((m, n) => Math.min(m, n.pitch), 100) : 0;
  const maxPitch = currentPhrase ? currentPhrase.notes.reduce((m, n) => Math.max(m, n.pitch), -100) : 1;
  const pitchRange = Math.max(maxPitch - minPitch, 14); // Provide vertical padding

  const renderPhrasePianoRoll = (phrase: Phrase, isActivePhrase: boolean) => {
    // Show 4 seconds of lead-in before the phrase starts
    const leadInBeats = Math.max(16, Math.ceil(4 / beatDuration));
    const viewStartBeat = phrase.startBeat - leadInBeats;
    const totalBeatsView = Math.max(phrase.endBeat - viewStartBeat, 10); 
    
    const sweeperPercent = ((currentBeat - viewStartBeat) / totalBeatsView) * 100;
    const isSweeperVisible = isActivePhrase && sweeperPercent >= 0 && currentBeat <= phrase.endBeat;

    return (
      <div className={`relative w-full h-full transition-opacity duration-500 ${isActivePhrase ? 'opacity-100' : 'opacity-0 hidden'}`}>
        
        {/* Pitch Lines Background */}
        <div className="absolute inset-0 flex flex-col justify-between opacity-20 pointer-events-none">
          {[...Array(12)].map((_, i) => (
            <div key={i} className="w-full h-px bg-slate-400" />
          ))}
        </div>

        {phrase.notes.map((note, idx) => {
          const isNoteActive = currentBeat >= note.beat && currentBeat < (note.beat + note.length);
          const isNotePast = currentBeat >= (note.beat + note.length);
          
          const leftPercent = ((note.beat - viewStartBeat) / totalBeatsView) * 100; 
          const widthPercent = (note.length / totalBeatsView) * 100;
          
          const pitchOffset = note.pitch - minPitch;
          const topPercent = 100 - (((pitchOffset + 2) / (pitchRange + 4)) * 100); 

          // USDX Style 3D Pills
          let baseGradient = 'from-slate-400 via-slate-300 to-slate-500';
          let borderColor = 'border-slate-500';
          if (note.note_type === '*') {
            baseGradient = 'from-yellow-400 via-yellow-200 to-yellow-600';
            borderColor = 'border-yellow-600';
          }
          
          return (
            <div
              key={idx}
              className={`absolute rounded-full transition-all duration-100 flex items-center justify-center overflow-hidden
                border-2 ${borderColor} bg-gradient-to-b ${baseGradient} shadow-[inset_0_2px_4px_rgba(255,255,255,0.5),0_4px_8px_rgba(0,0,0,0.5)]
                ${isNoteActive ? 'scale-105 z-10 ring-2 ring-white/50' : 'z-0'}
              `}
              style={{
                left: `${leftPercent}%`,
                width: `${Math.max(widthPercent, 2)}%`,
                top: `${topPercent}%`,
                height: '24px',
                marginTop: '-12px',
                opacity: isNotePast ? 0.7 : 1,
              }}
            >
              {/* Note Fill (If passed and hit, wait we don't have perfect history for past notes easily here, 
                  but we can just draw the pitch history blocks on top. ) */}
            </div>
          );
        })}

        {/* Pitch History Blocks (Sung pitch) */}
        {isActivePhrase && pitchHistory.map((pt, i) => {
          if (pt.beat < viewStartBeat || pt.beat > phrase.endBeat) return null;

          let targetPitch = minPitch + (pitchRange / 2);
          const activeNote = phrase.notes.find(n => pt.beat >= n.beat && pt.beat < n.beat + n.length);
          if (activeNote) targetPitch = activeNote.pitch;

          const noteClassTarget = (targetPitch % 12 + 12) % 12;
          const noteClassSung = (pt.pitch % 12 + 12) % 12;
          const classDiff = noteClassSung - noteClassTarget;
          
          let normalizedPitch = targetPitch;
          if (classDiff > 6) normalizedPitch = targetPitch + classDiff - 12;
          else if (classDiff < -6) normalizedPitch = targetPitch + classDiff + 12;
          else normalizedPitch = targetPitch + classDiff;

          const ptLeftPercent = ((pt.beat - viewStartBeat) / totalBeatsView) * 100;
          const pitchOffset = normalizedPitch - minPitch;
          const ptTopPercent = 100 - (((pitchOffset + 2) / (pitchRange + 4)) * 100);

          // Hit color = Blue, Miss color = Grey
          const blockColor = pt.isHit ? 'from-blue-400 to-blue-600' : 'from-slate-400 to-slate-600';

          return (
            <div 
              key={i}
              className={`absolute h-5 rounded-sm bg-gradient-to-b ${blockColor} shadow-md z-30 opacity-90`}
              style={{
                left: `calc(${ptLeftPercent}% - 6px)`,
                top: `calc(${ptTopPercent}% - 10px)`,
                width: '12px'
              }}
            />
          );
        })}

        {/* Countdown Timer */}
        {isActivePhrase && currentBeat < phrase.startBeat && currentBeat >= viewStartBeat && (
          <div 
            className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 text-white/50 font-black italic text-6xl md:text-8xl z-0 animate-pulse drop-shadow-[0_4px_10px_rgba(0,0,0,0.8)]"
          >
            {Math.ceil((phrase.startBeat - currentBeat) * beatDuration)}
          </div>
        )}

        {/* Progress Sweeper */}
        {isSweeperVisible && (
          <div 
            className="absolute top-0 bottom-0 w-1 bg-white shadow-[0_0_20px_rgba(255,255,255,1)] z-40"
            style={{ left: `${sweeperPercent}%` }}
          />
        )}
      </div>
    );
  };

  const renderLyrics = (phrase: Phrase | undefined, isNext: boolean) => {
    if (!phrase) return null;
    return (
      <div className={`flex flex-wrap justify-center gap-x-1 font-bold px-4 
        ${isNext ? 'text-xl md:text-2xl text-slate-400' : 'text-3xl md:text-5xl text-white'}`}
      >
        {phrase.notes.map((note, idx) => {
          const isNotePast = currentBeat >= (note.beat + note.length);
          const isNoteActive = currentBeat >= note.beat && !isNotePast;
          
          let colorClass = '';
          if (!isNext) {
            if (isNotePast || isNoteActive) {
              colorClass = 'text-blue-400 drop-shadow-[0_0_8px_rgba(59,130,246,0.8)]';
            } else {
              colorClass = 'text-white drop-shadow-[0_2px_4px_rgba(0,0,0,0.8)]';
            }
          }

          const text = note.text.replace(/~/g, '').replace(/ /g, '\u00A0');

          return (
            <span key={idx} className={`transition-colors duration-100 ${colorClass}`}>
              {text}
            </span>
          );
        })}
      </div>
    );
  };

  const isFinished = currentTime >= metadata.total_duration - 1.0;

  const [autoContinueCancelled, setAutoContinueCancelled] = useState(false);
  const [endCountdown, setEndCountdown] = useState(10);

  useEffect(() => {
    if (isFinished && !autoContinueCancelled) {
      if (endCountdown <= 0) {
        if (onFinish) onFinish();
        return;
      }
      const timer = setTimeout(() => {
        setEndCountdown(c => c - 1);
      }, 1000);
      return () => clearTimeout(timer);
    }
  }, [isFinished, autoContinueCancelled, endCountdown, onFinish]);

  const getRank = (score: number) => {
    if (score < 2000) return { title: 'Tone Deaf', color: 'text-gray-400' };
    if (score < 4000) return { title: 'Amateur', color: 'text-blue-400' };
    if (score < 5000) return { title: 'Wannabe', color: 'text-teal-400' };
    if (score < 6000) return { title: 'Hopeful', color: 'text-green-400' };
    if (score < 7000) return { title: 'Rising Star', color: 'text-yellow-400' };
    if (score < 8000) return { title: 'Lead Singer', color: 'text-orange-400' };
    if (score < 9000) return { title: 'Superstar', color: 'text-purple-400' };
    return { title: 'Ultrastar!', color: 'text-pink-500 animate-pulse' };
  };

  const rank = getRank(score);

  return (
    <div className="flex flex-col h-full w-full bg-slate-950 font-sans relative">
      
      {/* Background Media */}
      {mediaUrl && (
        <div className="absolute inset-0 z-0 overflow-hidden bg-black">
          {mediaUrl.type === 'video' ? (
            <video 
              src={mediaUrl.url} 
              muted 
              className="w-full h-full object-cover opacity-50"
              ref={(ref) => {
                if (ref) {
                  if (isPlaying && ref.paused) ref.play().catch(()=>{});
                  if (!isPlaying && !ref.paused) ref.pause();
                  // Sync if drift > 0.5s
                  if (Math.abs(ref.currentTime - currentTime) > 0.5) {
                    ref.currentTime = currentTime;
                  }
                }
              }}
            />
          ) : (
            <img src={mediaUrl.url} alt="Cover" className="w-full h-full object-cover opacity-50" />
          )}
        </div>
      )}

      {/* SCORE BOX (Top Right) */}
      <div className="absolute top-4 right-4 z-50 bg-gradient-to-b from-blue-500 to-blue-700 border-2 border-blue-400 rounded-lg shadow-[0_4px_15px_rgba(0,0,0,0.5)] px-6 py-2 flex items-center justify-center">
        <span className="text-white font-black text-3xl tracking-widest drop-shadow-md">
          {Math.floor(score).toString().padStart(5, '0')}
        </span>
      </div>

      <div className="flex-1 relative overflow-hidden flex flex-col pt-12 pb-32">
        {/* Piano Roll Area */}
        <div className="flex-1 relative w-full px-8">
          {currentPhrase && renderPhrasePianoRoll(currentPhrase, true)}
        </div>
      </div>

      {/* Lyrics Area (USDX Style Bottom Box) */}
      <div className="absolute bottom-0 left-0 right-0 h-40 bg-gradient-to-t from-slate-900 via-slate-800/90 to-transparent flex flex-col justify-end pb-8 pt-12 border-t border-white/10 shadow-[0_-10px_20px_rgba(0,0,0,0.5)]">
        <div className="flex flex-col items-center justify-center gap-2">
          {renderLyrics(currentPhrase, false)}
          <div className="h-8 mt-1">
            {renderLyrics(nextPhrase, true)}
          </div>
        </div>
      </div>

      {/* Progress Bar (Bottom Edge) */}
      <div className="absolute bottom-0 left-0 right-0 h-1 bg-slate-800 z-50">
        <div
          className="h-full bg-blue-500 transition-all duration-200"
          style={{ width: `${(currentTime / metadata.total_duration) * 100}%` }}
        />
      </div>

      {/* END SCREEN OVERLAY */}
      {isFinished && (
        <div 
          className="absolute inset-0 z-50 bg-black/80 backdrop-blur-md flex flex-col items-center justify-center p-8 animate-in fade-in duration-1000"
          onMouseMove={() => setAutoContinueCancelled(true)}
        >
          <h2 className="text-4xl text-white font-bold mb-2 drop-shadow-lg">{metadata.title}</h2>
          <p className="text-xl text-slate-300 mb-8">{metadata.artist}</p>
          
          <div className="bg-slate-900/80 border border-slate-700 p-8 rounded-2xl shadow-2xl flex flex-col items-center gap-6 min-w-[400px]">
            <h3 className="text-slate-400 uppercase tracking-widest text-sm font-bold">Final Score</h3>
            <div className="text-7xl font-black text-white drop-shadow-[0_0_15px_rgba(255,255,255,0.5)] tracking-widest">
              {Math.floor(score).toString().padStart(5, '0')}
            </div>
            
            <div className="w-full h-px bg-slate-800 my-2" />
            
            <h3 className="text-slate-400 uppercase tracking-widest text-sm font-bold">Ranking</h3>
            <div className={`text-5xl font-black drop-shadow-md ${rank.color}`}>
              {rank.title}
            </div>
          </div>

          <button 
            onClick={() => onFinish && onFinish()}
            className="mt-12 px-8 py-4 bg-blue-600 hover:bg-blue-500 transition-colors text-white font-bold rounded-full text-xl shadow-[0_0_20px_rgba(59,130,246,0.6)]"
          >
            Continue {(!autoContinueCancelled && endCountdown > 0) ? `(${endCountdown}s)` : ''}
          </button>
        </div>
      )}
    </div>
  );
};

export default UltrastarDisplay;
