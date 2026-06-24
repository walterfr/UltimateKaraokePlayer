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

const formatBytes = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
};

const TrackerDisplay: React.FC<TrackerDisplayProps> = ({ metadata, isPlaying, currentTime }) => {
  if (!metadata) {
    return (
      <div className="flex items-center justify-center h-full text-slate-500">
        <p className="text-lg italic">No tracker module loaded</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full w-full bg-gradient-to-br from-slate-900 via-slate-800 to-slate-900">
      {/* Header */}
      <div className="text-center px-6 py-6 border-b border-slate-700/50">
        <div className="flex items-center justify-center gap-2 text-emerald-400 mb-1">
          <Disc size={18} />
          <span className="text-xs font-bold tracking-widest uppercase">{metadata.format_name}</span>
        </div>
        <h2 className="text-2xl font-bold text-white tracking-tight">
          {metadata.title || 'Untitled Module'}
        </h2>
      </div>

      {/* Stats grid */}
      <div className="grid grid-cols-2 gap-3 p-4">
        <div className="bg-slate-800/50 rounded-lg p-3 border border-slate-700/30">
          <div className="flex items-center gap-2 text-emerald-400 text-xs mb-1">
            <Hash size={14} />
            <span>Channels</span>
          </div>
          <span className="text-2xl font-bold text-white">{metadata.channel_count}</span>
        </div>
        <div className="bg-slate-800/50 rounded-lg p-3 border border-slate-700/30">
          <div className="flex items-center gap-2 text-emerald-400 text-xs mb-1">
            <Layers size={14} />
            <span>Patterns</span>
          </div>
          <span className="text-2xl font-bold text-white">{metadata.pattern_count}</span>
        </div>
        <div className="bg-slate-800/50 rounded-lg p-3 border border-slate-700/30">
          <div className="flex items-center gap-2 text-emerald-400 text-xs mb-1">
            <Music size={14} />
            <span>Instruments</span>
          </div>
          <span className="text-2xl font-bold text-white">{metadata.instrument_count}</span>
        </div>
        <div className="bg-slate-800/50 rounded-lg p-3 border border-slate-700/30">
          <div className="flex items-center gap-2 text-emerald-400 text-xs mb-1">
            <Volume2 size={14} />
            <span>File Size</span>
          </div>
          <span className="text-xl font-bold text-white">{formatBytes(metadata.file_size)}</span>
        </div>
      </div>

      {/* Instruments list */}
      {metadata.instruments.length > 0 && (
        <div className="flex-1 overflow-y-auto px-4 pb-4">
          <div className="flex items-center gap-2 text-emerald-400 text-xs font-bold mb-2 uppercase tracking-wider">
            <FileText size={14} />
            <span>Instruments</span>
          </div>
          <div className="space-y-1">
            {metadata.instruments.map((inst, i) => (
              <div key={i} className="flex items-center justify-between bg-slate-800/30 rounded px-3 py-1.5 text-sm">
                <span className="text-slate-300 truncate mr-2">
                  {i + 1}. {inst.name || `Instrument ${i + 1}`}
                </span>
                <span className="text-slate-500 text-xs">{inst.sample_count} samples</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Playing indicator */}
      {isPlaying && (
        <div className="h-1 bg-emerald-500/30">
          <div 
            className="h-full bg-emerald-400 transition-all duration-200"
            style={{ width: `${((currentTime % 60) / 60) * 100}%` }}
          />
        </div>
      )}
    </div>
  );
};

export default TrackerDisplay;
