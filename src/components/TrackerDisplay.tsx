import React from 'react';
import { Music, Disc, Hash, Volume2, Layers, FileText } from 'lucide-react';

export interface TrackerInstrument {
  name: string;
  sample_count: number;
}

export interface TrackerMetadata {
  title: string;
  format_name: string;
  channel_count: number;
  pattern_count: number;
  instrument_count: number;
  estimated_duration: number;
  instruments: TrackerInstrument[];
  file_size: number;
  tracker_message: string;
}

interface TrackerDisplayProps {
  metadata: TrackerMetadata | null;
  isPlaying: boolean;
  currentTime: number;
}

const formatBytes = (bytes: number | undefined): string => {
  if (bytes === undefined) return "0 B";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
};

const TrackerDisplay: React.FC<TrackerDisplayProps> = ({ metadata, isPlaying, currentTime }) => {
  if (!metadata) {
    return (
      <div className="flex items-center justify-center h-full w-full bg-slate-900 text-slate-400">
        <div className="flex flex-col items-center gap-4">
          <Disc size={48} className="animate-spin-slow opacity-50" />
          <p className="text-xl italic">Loading tracker module...</p>
        </div>
      </div>
    );
  }

  // Fallback to 300s (5 minutes) if estimated_duration is 0 or missing
  const duration = (metadata.estimated_duration && metadata.estimated_duration > 0) 
    ? metadata.estimated_duration 
    : 300;
  
  const progressPercentage = Math.min(100, (currentTime / duration) * 100);

  return (
    <div className="flex flex-col h-full w-full bg-gradient-to-br from-slate-900 via-slate-800 to-slate-900 relative overflow-hidden shadow-inner">
      {/* Background ambient lighting */}
      <div className="absolute inset-0 pointer-events-none opacity-50">
        <div className="absolute -top-[20%] -left-[10%] w-[60vw] h-[60vw] bg-emerald-600/30 rounded-full blur-[100px] mix-blend-screen animate-[pulse_8s_ease-in-out_infinite]" />
        <div className="absolute -bottom-[20%] -right-[10%] w-[70vw] h-[70vw] bg-teal-600/30 rounded-full blur-[120px] mix-blend-screen animate-[pulse_12s_ease-in-out_infinite_reverse]" />
      </div>
      
      <div className="relative z-10 flex flex-col h-full w-full">
        {/* Header */}
        <div className="text-center px-6 py-6 border-b border-slate-700/50 bg-slate-900/40 backdrop-blur-sm">
          <div className="flex items-center justify-center gap-2 text-emerald-400 mb-2">
            <Disc size={20} className={isPlaying ? "animate-spin" : ""} />
            <span className="text-sm font-bold tracking-widest uppercase bg-slate-800 px-3 py-1 rounded-full border border-emerald-500/30">
              {metadata.format_name || 'TRACKER'}
            </span>
          </div>
          <h2 className="text-3xl font-extrabold text-white tracking-tight drop-shadow-md">
            {metadata.title || 'Untitled Module'}
          </h2>
        </div>

        {/* Stats grid */}
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-4 p-6">
          <div className="bg-slate-800/60 rounded-xl p-4 border border-slate-600/50 shadow-lg flex flex-col items-center justify-center backdrop-blur-sm">
            <div className="flex items-center gap-2 text-emerald-400 text-sm mb-2 font-medium">
              <Hash size={16} />
              <span>Channels</span>
            </div>
            <span className="text-3xl font-bold text-white">{metadata.channel_count || 0}</span>
          </div>
          <div className="bg-slate-800/60 rounded-xl p-4 border border-slate-600/50 shadow-lg flex flex-col items-center justify-center backdrop-blur-sm">
            <div className="flex items-center gap-2 text-emerald-400 text-sm mb-2 font-medium">
              <Layers size={16} />
              <span>Patterns</span>
            </div>
            <span className="text-3xl font-bold text-white">{metadata.pattern_count || 0}</span>
          </div>
          <div className="bg-slate-800/60 rounded-xl p-4 border border-slate-600/50 shadow-lg flex flex-col items-center justify-center backdrop-blur-sm">
            <div className="flex items-center gap-2 text-emerald-400 text-sm mb-2 font-medium">
              <Music size={16} />
              <span>Instruments</span>
            </div>
            <span className="text-3xl font-bold text-white">{metadata.instrument_count || 0}</span>
          </div>
          <div className="bg-slate-800/60 rounded-xl p-4 border border-slate-600/50 shadow-lg flex flex-col items-center justify-center backdrop-blur-sm">
            <div className="flex items-center gap-2 text-emerald-400 text-sm mb-2 font-medium">
              <Volume2 size={16} />
              <span>File Size</span>
            </div>
            <span className="text-2xl font-bold text-white">{formatBytes(metadata.file_size)}</span>
          </div>
        </div>

        {/* Instruments list */}
        {metadata.instruments && metadata.instruments.length > 0 && (
          <div className="flex-1 overflow-y-auto px-6 pb-6">
            <div className="bg-slate-900/40 rounded-xl p-4 border border-slate-700/50 backdrop-blur-sm h-full flex flex-col">
              <div className="flex items-center gap-2 text-emerald-400 text-sm font-bold mb-4 uppercase tracking-wider">
                <FileText size={16} />
                <span>Instrument Roster</span>
              </div>
              <div className="space-y-1.5 overflow-y-auto pr-2 custom-scrollbar flex-1">
                {metadata.instruments.map((inst, i) => (
                  <div key={i} className="flex items-center justify-between bg-slate-800/50 hover:bg-slate-700/50 transition-colors rounded-lg px-4 py-2 text-sm border border-slate-700/30">
                    <span className="text-slate-200 font-medium truncate mr-4">
                      <span className="text-emerald-500/70 mr-2">{String(i + 1).padStart(2, '0')}.</span>
                      {inst.name || `Instrument ${i + 1}`}
                    </span>
                    <span className="text-slate-400 text-xs bg-slate-900/50 px-2 py-1 rounded">
                      {inst.sample_count} smp
                    </span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}

        {/* Playing indicator */}
        <div className="h-2 bg-slate-900 shrink-0 w-full relative">
          {isPlaying && (
            <div 
              className="absolute top-0 left-0 h-full bg-gradient-to-r from-emerald-500 via-teal-400 to-emerald-400 transition-all duration-200 ease-linear shadow-[0_0_10px_rgba(52,211,153,0.5)]"
              style={{ width: `${progressPercentage}%` }}
            />
          )}
        </div>
      </div>
    </div>
  );
};

export default TrackerDisplay;
