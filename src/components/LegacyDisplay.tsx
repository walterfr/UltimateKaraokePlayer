import React from 'react';
import { FileQuestion, AlertTriangle, Info, FileText } from 'lucide-react';

export interface LegacyMetadata {
  title: string;
  artist: string;
  format: string;
  file_size: number;
  detected_subformat: string;
  estimated_duration: number;
  header_hex: string;
  notes: string;
}

interface LegacyDisplayProps {
  metadata: LegacyMetadata | null;
  filePath?: string;
}

const LegacyDisplay: React.FC<LegacyDisplayProps> = ({ metadata, filePath }) => {
  if (!metadata) {
    return (
      <div className="flex items-center justify-center h-full text-slate-500">
        <p className="text-lg italic">No legacy file loaded</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full w-full bg-gradient-to-br from-slate-900 via-amber-950/20 to-slate-900">
      {/* Header */}
      <div className="text-center px-6 py-6 border-b border-amber-800/30">
        <div className="flex items-center justify-center gap-2 text-amber-400 mb-1">
          <FileQuestion size={18} />
          <span className="text-xs font-bold tracking-widest uppercase">{metadata.format}</span>
        </div>
        <h2 className="text-2xl font-bold text-white tracking-tight">
          {metadata.title}
        </h2>
        {metadata.artist && (
          <p className="text-amber-300/70 text-sm mt-1">{metadata.artist}</p>
        )}
      </div>

      {/* Info cards */}
      <div className="grid grid-cols-2 gap-3 p-4">
        <div className="bg-amber-900/20 rounded-lg p-3 border border-amber-700/30">
          <div className="flex items-center gap-2 text-amber-400 text-xs mb-1">
            <Info size={14} />
            <span>Detected As</span>
          </div>
          <span className="text-lg font-bold text-white capitalize">{metadata.detected_subformat.replace('_', ' ')}</span>
        </div>
        <div className="bg-amber-900/20 rounded-lg p-3 border border-amber-700/30">
          <div className="flex items-center gap-2 text-amber-400 text-xs mb-1">
            <FileText size={14} />
            <span>File Size</span>
          </div>
          <span className="text-lg font-bold text-white">
            {metadata.file_size < 1024 ? `${metadata.file_size} B` :
             metadata.file_size < 1048576 ? `${(metadata.file_size/1024).toFixed(1)} KB` :
             `${(metadata.file_size/1048576).toFixed(1)} MB`}
          </span>
        </div>
      </div>

      {/* Notes / Compatibility warning */}
      <div className="px-4 mb-4">
        <div className="flex items-start gap-2 bg-amber-900/20 rounded-lg p-3 border border-amber-700/30">
          <AlertTriangle size={16} className="text-amber-400 mt-0.5 shrink-0" />
          <p className="text-sm text-amber-200/80 leading-relaxed">
            {metadata.notes || "Legacy format detected. Full playback may require conversion."}
          </p>
        </div>
      </div>

      {/* Header hex dump */}
      <div className="flex-1 px-4 pb-4 overflow-hidden">
        <div className="flex items-center gap-2 text-amber-400 text-xs font-bold mb-2 uppercase tracking-wider">
          <FileText size={14} />
          <span>Header Raw (first 32 bytes)</span>
        </div>
        <div className="bg-black/40 rounded-lg p-3 font-mono text-[11px] text-slate-400 leading-relaxed overflow-x-auto border border-slate-700/30">
          {metadata.header_hex}
        </div>
      </div>

      {/* File path */}
      {filePath && (
        <div className="px-4 pb-4">
          <div className="text-[10px] uppercase tracking-wider text-slate-600 mb-1">Path</div>
          <div className="text-xs text-slate-500 truncate bg-black/20 rounded px-2 py-1">{filePath}</div>
        </div>
      )}
    </div>
  );
};

export default LegacyDisplay;
