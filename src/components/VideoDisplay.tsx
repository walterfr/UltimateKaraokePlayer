import React, { useRef, useState, useEffect } from 'react';

export interface SubtitleCue {
  text: string;
  start_seconds: number;
  end_seconds: number;
  style?: string;
}

interface VideoDisplayProps {
  videoPath: string;
  subtitles: SubtitleCue[];
  isPlaying: boolean;
  onTimeUpdate?: (time: number) => void;
  onDuration?: (duration: number) => void;
}

const VideoDisplay: React.FC<VideoDisplayProps> = ({
  videoPath,
  subtitles,
  isPlaying,
  onTimeUpdate,
  onDuration,
}) => {
  const videoRef = useRef<HTMLVideoElement>(null);
  const [currentSubtitle, setCurrentSubtitle] = useState<string>('');

  // Sync video play/pause with app state
  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;
    if (isPlaying) {
      video.play().catch(() => {});
    } else {
      video.pause();
    }
  }, [isPlaying]);

  // Track current subtitle based on video time
  const handleTimeUpdate = () => {
    const video = videoRef.current;
    if (!video) return;

    const time = video.currentTime;
    onTimeUpdate?.(time);

    // Find matching subtitle
    let found = '';
    for (const sub of subtitles) {
      if (time >= sub.start_seconds && time < sub.end_seconds) {
        found = sub.text;
        break;
      }
    }
    setCurrentSubtitle(found);
  };

  const handleLoadedMetadata = () => {
    const video = videoRef.current;
    if (video) {
      onDuration?.(video.duration);
    }
  };

  return (
    <div className="relative w-full h-full bg-black flex items-center justify-center overflow-hidden">
      <video
        ref={videoRef}
        className="w-full h-full object-contain"
        onTimeUpdate={handleTimeUpdate}
        onLoadedMetadata={handleLoadedMetadata}
        preload="metadata"
        // Convert file path to a local URL the webview can access
        // In Tauri, we can use asset protocol or convertFileSrc
        src={videoPath}
      />

      {/* Subtitle overlay */}
      {currentSubtitle && (
        <div className="absolute bottom-16 left-0 right-0 text-center px-6 pointer-events-none">
          <span className="inline-block bg-black/70 text-white text-2xl font-bold px-4 py-2 rounded-lg backdrop-blur-sm shadow-2xl leading-relaxed max-w-3xl mx-auto">
            {currentSubtitle.split('\n').map((line, i) => (
              <span key={i}>
                {line}
                {i < currentSubtitle.split('\n').length - 1 && <br />}
              </span>
            ))}
          </span>
        </div>
      )}

      {/* Fallback when no video is loaded */}
      {!videoPath && (
        <div className="text-slate-500 text-lg italic">
          No video loaded
        </div>
      )}
    </div>
  );
};

export default VideoDisplay;
