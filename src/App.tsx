import React, { useState, useEffect, useRef } from 'react';
import { 
  Play, Pause, SkipBack, SkipForward, StopCircle, 
  Volume2, Mic, Music, Search, Settings, 
  Monitor, Users, ListMusic, Sliders, FileVideo
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/api/dialog';
import CdgCanvas from './components/CdgCanvas';
import SynthDisplay, { SynthMetadata } from './components/SynthDisplay';
import VideoDisplay, { SubtitleCue } from './components/VideoDisplay';
import TrackerDisplay, { TrackerMetadata } from './components/TrackerDisplay';
import LegacyDisplay, { LegacyMetadata } from './components/LegacyDisplay';
import UltrastarDisplay, { UltrastarMetadata } from './components/UltrastarDisplay';

const App = () => {
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState('00:00');
  const [currentTimeSec, setCurrentTimeSec] = useState(0);
  const [midiMetadata, setMidiMetadata] = useState<SynthMetadata | null>(null);
  const [playMode, setPlayMode] = useState<'idle' | 'cdg' | 'midi' | 'video' | 'tracker' | 'legacy' | 'ultrastar'>('idle');
  // Video state
  const [videoPath, setVideoPath] = useState('');
  const [videoSubtitles, setVideoSubtitles] = useState<SubtitleCue[]>([]);
  // Tracker state
  const [trackerMetadata, setTrackerMetadata] = useState<TrackerMetadata | null>(null);
  // Legacy state
  const [legacyMetadata, setLegacyMetadata] = useState<LegacyMetadata | null>(null);
  // Ultrastar state
  const [ultrastarMetadata, setUltrastarMetadata] = useState<UltrastarMetadata | null>(null);
  // Current file path display
  const [currentFilePath, setCurrentFilePath] = useState('');
  // Library state
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<any[]>([]);
  const [sortBy, setSortBy] = useState<'title' | 'artist' | 'type' | 'recent'>('title');
  const [queueItems, setQueueItems] = useState<any[]>([]);
  const [songCount, setSongCount] = useState(0);
  const [scanStatus, setScanStatus] = useState('');
  const [libraryReady, setLibraryReady] = useState(false);
  const [engineFolders, setEngineFolders] = useState<Record<string, string>>({});
  const [inputDevices, setInputDevices] = useState<MediaDeviceInfo[]>([]);
  const [selectedInputDevice, setSelectedInputDevice] = useState<string>('none');
  const [outputDevices, setOutputDevices] = useState<string[]>([]);

  // Initialize library
  useEffect(() => {
    (async () => {
      try {
        const count: number = await invoke('get_songs_count');
        setSongCount(count);
        setLibraryReady(true);
        const q: any[] = await invoke('get_queue');
        setQueueItems(q);
        const settings: Record<string, string> = await invoke('get_all_settings');
        setEngineFolders(settings);
        // Add default library load
        const all: any[] = await invoke('search_songs', { query: '', sort_by: 'title' });
        setSearchResults(all);
      } catch (e) {
        console.warn('Library not available:', e);
      }
    })();

    // Carregar lista de output devices
    (async () => {
      try {
        // Fetch output devices from Rust
        invoke('list_output_devices').then((devices: any) => {
          setOutputDevices(devices);
        }).catch(console.error);

        // Fetch input devices from Browser WebRTC
        navigator.mediaDevices.enumerateDevices().then(devices => {
          const audioInputs = devices.filter(d => d.kind === 'audioinput');
          setInputDevices(audioInputs);
          if (audioInputs.length > 0 && selectedInputDevice === 'none') {
            setSelectedInputDevice(audioInputs[0].deviceId);
          }
        }).catch(console.error);
      } catch (e) {
        console.error('Erro ao listar output devices:', e);
      }
    })();
  }, []);

  // Auto-refresh queue every 3s while open
  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        const q: any[] = await invoke('get_queue');
        setQueueItems(q);
      } catch {}
    }, 3000);

    // Escuta eventos instantâneos do backend (ex: Cliente Remoto)
    const unlisten = listen('queue_updated', async () => {
      try {
        const q: any[] = await invoke('get_queue');
        setQueueItems(q);
      } catch {}
    });

    return () => {
      clearInterval(interval);
      unlisten.then(f => f());
    };
  }, []);

  const handlePlayToggle = async () => {
    if (!isPlaying) {
      if (playMode !== 'idle') {
        await invoke('resume_audio');
        setIsPlaying(true);
      } else {
        await playNextInQueue();
      }
    } else {
      await invoke('pause_audio');
      setIsPlaying(false);
    }
  };

  // Playback timer for all non-video modes
  const timerRef = useRef<number | null>(null);
  useEffect(() => {
    if (isPlaying && playMode !== 'video' && playMode !== 'idle') {
      timerRef.current = window.setInterval(() => {
        setCurrentTimeSec(prev => {
          const next = prev + 0.05;
          let max = 300;
          if (playMode === 'midi' && midiMetadata) max = midiMetadata.total_seconds;
          else if (playMode === 'ultrastar' && ultrastarMetadata) max = ultrastarMetadata.total_duration;
          else if (playMode === 'cdg') max = 600;
          if (next >= max) {
            setIsPlaying(false);
            if (playMode !== 'ultrastar') {
              playNextInQueue();
            }
            return max;
          }
          return next;
        });
      }, 50);
    } else {
      if (timerRef.current) { clearInterval(timerRef.current); timerRef.current = null; }
    }
    return () => { if (timerRef.current) clearInterval(timerRef.current); };
  }, [isPlaying, playMode, midiMetadata, ultrastarMetadata]);

  // Update displayed time from currentTimeSec
  useEffect(() => {
    const m = Math.floor(currentTimeSec / 60);
    const s = Math.floor(currentTimeSec % 60);
    setCurrentTime(`${m}:${s.toString().padStart(2, '0')}`);
  }, [currentTimeSec]);

  const setEngineFolder = async (engine: string, title: string) => {
    try {
      const selected = await open({ directory: true, title });
      
      let pathStr: string | null = null;
      if (typeof selected === 'string') {
        pathStr = selected;
      } else if (Array.isArray(selected) && selected.length > 0) {
        pathStr = selected[0];
      }

      if (!pathStr) {
        alert("Nenhuma pasta retornada pelo Windows! Retorno: " + JSON.stringify(selected));
        return;
      }

      try {
        await invoke('save_setting', { key: engine, value: pathStr });
        setEngineFolders(prev => ({ ...prev, [engine]: pathStr }));
        
        setScanStatus(`Scanning ${engine}...`);
        const result: any = await invoke('scan_library', { path: pathStr, engine });
        alert(`O motor Rust terminou de escanear. Ele encontrou: ${result.count} novas músicas!`);
        
        setScanStatus(`Found ${result.count} new songs`);
        setTimeout(() => setScanStatus(''), 5000);
        
        const count: number = await invoke('get_songs_count');
        setSongCount(count);
        const all: any[] = await invoke('search_songs', { query: '', sort_by: sortBy });
        setSearchResults(all);
      } catch (innerE: any) {
        alert("Erro IPC durante o scan: " + innerE.toString());
      }
    } catch (e: any) {
      alert("Erro crítico no dialog: " + e.toString());
      console.error('Scan error:', e);
      setScanStatus(`Scan failed: ${e.toString()}`);
    }
  };

  const searchSongs = async (query: string, sort: string = sortBy) => {
    setSearchQuery(query);
    try {
      const res: any[] = await invoke('search_songs', { query, sort_by: sort });
      setSearchResults(res);
    } catch (e: any) {
      console.error('Search error:', e);
      setScanStatus(`Search failed: ${e}`);
    }
  };

  const clearLibrary = async () => {
    if (confirm("Are you sure you want to clear the entire library? This won't delete your files.")) {
      try {
        await invoke('clear_library');
        setSearchResults([]);
        setSongCount(0);
        setScanStatus('Library cleared');
        setTimeout(() => setScanStatus(''), 3000);
      } catch (e) {
        console.error(e);
      }
    }
  };

  const enqueueSong = async (songId: number, title: string) => {
    try {
      await invoke('enqueue_song', { songId, requestedBy: '' });
      const q: any[] = await invoke('get_queue');
      setQueueItems(q);
    } catch (e) {
      console.error('Enqueue error:', e);
    }
  };

  const removeFromQueue = async (queueId: number) => {
    try {
      await invoke('remove_from_queue', { queueId });
      const q: any[] = await invoke('get_queue');
      setQueueItems(q);
    } catch {}
  };

  const clearQueue = async () => {
    try {
      await invoke('clear_queue');
      setQueueItems([]);
    } catch {}
  };

  const playQueueItem = async (item: any) => {
    const song = item.song;
    if (!song) return;
    
    try {
      const ext = song.file_path.split('.').pop()?.toLowerCase();
      
      // Stop currently playing audio first
      await invoke('stop_audio');
      
      if (ext === 'mp4' || ext === 'mkv' || ext === 'avi' || ext === 'mov') {
        let videoUrl = song.file_path;
        try {
          const { convertFileSrc } = await import('@tauri-apps/api/tauri');
          videoUrl = convertFileSrc(song.file_path);
        } catch (e) {}
        const result: any = await invoke('parse_video_file', { path: song.file_path });
        setVideoSubtitles(result.subtitles || []);
        setVideoPath(videoUrl);
        setPlayMode('video');
      } else if (ext === 'mid' || ext === 'kar') {
        const result: SynthMetadata = await invoke('parse_midi_file', { path: song.file_path });
        setMidiMetadata(result);
        setPlayMode('midi');
        await invoke('play_song', { path: song.file_path });
      } else if (ext === 'mod' || ext === 'xm' || ext === 's3m' || ext === 'st3' || ext === 'it') {
        const result: TrackerMetadata = await invoke('parse_tracker_file', { path: song.file_path });
        setTrackerMetadata(result);
        setPlayMode('tracker');
      } else if (ext === 'txt') {
        const result: UltrastarMetadata = await invoke('parse_ultrastar_file', { path: song.file_path });
        setUltrastarMetadata(result);
        setPlayMode('ultrastar');
        if (result.mp3) {
          // Resolve o caminho absoluto do MP3 na mesma pasta do txt
          const dir = song.file_path.replace(/\\/g, '/').substring(0, song.file_path.replace(/\\/g, '/').lastIndexOf('/'));
          const audioPath = `${dir}/${result.mp3}`;
          await invoke('play_song', { path: audioPath });
        }
      } else {
        setPlayMode('cdg');
        await invoke('play_song', { path: song.file_path });
      }
      
      setCurrentFilePath(song.file_path);
      setCurrentTimeSec(0);
      setIsPlaying(true);
      
      // Remove from queue since it's playing
      await removeFromQueue(item.id);
    } catch (e) {
      console.error('Error playing queue item:', e);
    }
  };

  const playNextInQueue = async () => {
    try {
      const q: any[] = await invoke('get_queue');
      if (q.length > 0) {
        await playQueueItem(q[0]);
      } else {
        // No more items, just stop
        await invoke('stop_audio');
        setIsPlaying(false);
        setCurrentTimeSec(0);
        setPlayMode('idle');
      }
    } catch (e) {
      console.error(e);
    }
  };

  const getEngineColor = (type: string) => {
    switch(type) {
      case 'cdg': return 'bg-blue-900/50 text-blue-300 border-blue-800';
      case 'video': return 'bg-purple-900/50 text-purple-300 border-purple-800';
      case 'midi': return 'bg-orange-900/50 text-orange-300 border-orange-800';
      case 'tracker': return 'bg-emerald-900/50 text-emerald-300 border-emerald-800';
      case 'legacy': return 'bg-amber-900/50 text-amber-300 border-amber-800';
      case 'ultrastar': return 'bg-pink-900/50 text-pink-300 border-pink-800';
      default: return 'bg-slate-800 text-slate-300 border-slate-700';
    }
  };

  return (
    <div className="h-screen w-screen flex flex-col bg-karaoke-dark text-karaoke-text select-none overflow-hidden">
      {/* Main Content Area */}
      <div className="flex-1 flex overflow-hidden">
        
        {/* Left Panel: Settings & Mixing */}
        <div className="w-80 bg-karaoke-panel border-r border-slate-700 flex flex-col p-4 gap-6 overflow-y-auto">
          <div className="flex items-center gap-2 text-blue-400 font-bold mb-2">
            <FileVideo size={20} />
            <h2>LOAD MEDIA</h2>
          </div>
          <div className="space-y-2">
            <button onClick={() => setEngineFolder('cdg', 'Select MP3/CDG Folder')} className="w-full p-2 bg-slate-800 hover:bg-blue-600 rounded-lg text-xs font-bold transition-colors border border-slate-700 text-left">
              🎵 Configure CDG Folder
              {engineFolders.cdg && <div className="text-[9px] font-normal text-slate-400 truncate mt-1">{engineFolders.cdg}</div>}
            </button>
            <button onClick={() => setEngineFolder('video', 'Select Video Folder')} className="w-full p-2 bg-slate-800 hover:bg-blue-600 rounded-lg text-xs font-bold transition-colors border border-slate-700 text-left">
              🎬 Configure Video Folder
              {engineFolders.video && <div className="text-[9px] font-normal text-slate-400 truncate mt-1">{engineFolders.video}</div>}
            </button>
            <button onClick={() => setEngineFolder('midi', 'Select MIDI Folder')} className="w-full p-2 bg-slate-800 hover:bg-blue-600 rounded-lg text-xs font-bold transition-colors border border-slate-700 text-left">
              🎹 Configure MIDI Folder
              {engineFolders.midi && <div className="text-[9px] font-normal text-slate-400 truncate mt-1">{engineFolders.midi}</div>}
            </button>
            <button onClick={() => setEngineFolder('tracker', 'Select Tracker Folder')} className="w-full p-2 bg-slate-800 hover:bg-emerald-600 rounded-lg text-xs font-bold transition-colors border border-slate-700 text-left">
              🎛️ Configure Tracker Folder
              {engineFolders.tracker && <div className="text-[9px] font-normal text-slate-400 truncate mt-1">{engineFolders.tracker}</div>}
            </button>
            <button onClick={() => setEngineFolder('legacy', 'Select Legacy Folder')} className="w-full p-2 bg-slate-800 hover:bg-amber-600 rounded-lg text-xs font-bold transition-colors border border-slate-700 text-left">
              🏚️ Configure Legacy Folder
              {engineFolders.legacy && <div className="text-[9px] font-normal text-slate-400 truncate mt-1">{engineFolders.legacy}</div>}
            </button>
            <button onClick={() => setEngineFolder('ultrastar', 'Select Ultrastar Folder')} className="w-full p-2 bg-slate-800 hover:bg-pink-600 rounded-lg text-xs font-bold transition-colors border border-slate-700 text-left">
              🎤 Configure Ultrastar Folder
              {engineFolders.ultrastar && <div className="text-[9px] font-normal text-slate-400 truncate mt-1">{engineFolders.ultrastar}</div>}
            </button>
          </div>

          {/* Library Scan */}
          <div className="px-1">
            {scanStatus && <div className="text-[10px] text-slate-400 mt-1">{scanStatus}</div>}
            <div className="text-[10px] text-slate-600 mt-1">{songCount} songs in library</div>
          </div>

          <div className="h-px bg-slate-700 my-2" />

          {/* File Path Display */}
          {currentFilePath && (
            <div className="px-1">
              <div className="text-[10px] uppercase tracking-widest text-slate-500 mb-1 font-bold">Loaded File</div>
              <div className="text-xs text-slate-400 truncate bg-slate-800/50 rounded px-2 py-1.5 border border-slate-700/30"
                   title={currentFilePath}>
                {currentFilePath.split('\\').pop()?.split('/').pop()}
              </div>
            </div>
          )}

          <div className="h-px bg-slate-700 my-2" />

          <div className="flex items-center gap-2 text-blue-400 font-bold mb-2">
            <Settings size={20} />
            <h2>AUDIO SETTINGS</h2>
          </div>
          
          <div className="space-y-4">
            <div className="space-y-2">
              <div className="flex justify-between text-sm opacity-70">
                <span>Key / Pitch</span>
                <span>0</span>
              </div>
              <input type="range" className="w-full accent-blue-500 h-1 bg-slate-600 rounded-lg appearance-none cursor-pointer" />
            </div>
            
            <div className="space-y-2">
              <div className="flex justify-between text-sm opacity-70">
                <span>Tempo</span>
                <span>1.0x</span>
              </div>
              <input type="range" className="w-full accent-blue-500 h-1 bg-slate-600 rounded-lg appearance-none cursor-pointer" />
            </div>
          </div>

          <div className="h-px bg-slate-700 my-2" />

          <div className="flex items-center gap-2 text-blue-400 font-bold mb-2">
            <Mic size={20} />
            <h2>RECORDING & FX</h2>
          </div>

          <div className="space-y-4">
            {[
              { label: 'Input Volume', icon: <Mic size={14}/> },
              { label: 'Echo', icon: <Sliders size={14}/> },
              { label: 'Reverb', icon: <Sliders size={14}/> },
              { label: 'Music Volume', icon: <Music size={14}/> },
              { label: 'Mic Delay', icon: <Settings size={14}/> },
            ].map((item) => (
              <div key={item.label} className="space-y-2">
                <div className="flex items-center gap-2 text-sm opacity-70">
                  {item.icon}
                  <span>{item.label}</span>
                </div>
                <input type="range" className="w-full accent-blue-500 h-1 bg-slate-600 rounded-lg appearance-none cursor-pointer" />
              </div>
            ))}
          </div>
        </div>

        {/* Center Panel: The Stage */}
        <div className="flex-1 relative bg-black flex items-center justify-center overflow-hidden group">
          {playMode === 'cdg' && (
            <div className="z-10 w-full h-full flex items-center justify-center p-8">
              <CdgCanvas />
            </div>
          )}
          {playMode === 'midi' && (
            <div className="z-10 w-full h-full">
              <SynthDisplay metadata={midiMetadata} isPlaying={isPlaying} currentTime={currentTimeSec} />
            </div>
          )}
          {playMode === 'video' && (
            <div className="z-10 w-full h-full">
              <VideoDisplay
                videoPath={videoPath}
                subtitles={videoSubtitles}
                isPlaying={isPlaying}
                onTimeUpdate={setCurrentTimeSec}
              />
            </div>
          )}
          {playMode === 'tracker' && (
            <div className="z-10 w-full h-full">
              <TrackerDisplay
                metadata={trackerMetadata}
                isPlaying={isPlaying}
                currentTime={currentTimeSec}
              />
            </div>
          )}
          {playMode === 'legacy' && (
            <div className="z-10 w-full h-full">
              <LegacyDisplay metadata={legacyMetadata} filePath={currentFilePath} />
            </div>
          )}
          {playMode === 'ultrastar' && (
            <div className="absolute inset-0 z-10 flex flex-col bg-slate-900 overflow-hidden pt-8">
              <UltrastarDisplay 
                metadata={ultrastarMetadata} 
                isPlaying={isPlaying} 
                currentTime={currentTimeSec} 
                micDeviceId={selectedInputDevice}
                songFilePath={currentFilePath}
                onFinish={playNextInQueue}
              />
            </div>
          )}
          {playMode === 'idle' && (
            <>
              {/* Background glow/animation placeholder */}
              <div className="absolute inset-0 bg-gradient-to-br from-blue-900/20 via-transparent to-purple-900/20 animate-pulse" />
              
              <div className="z-10 w-full h-full flex items-center justify-center p-8">
                <CdgCanvas />
              </div>
            </>
          )}

          {/* Overlays (only visible on hover) */}
          <div className="absolute top-4 right-4 opacity-0 group-hover:opacity-100 transition-opacity z-20">
            <button className="p-2 bg-slate-800/80 rounded-full hover:bg-blue-600 transition-colors">
              <Monitor size={20} />
            </button>
          </div>
        </div>

        {/* Right Panel: Library & Queue */}
        <div className="w-96 bg-karaoke-panel border-l border-slate-700 flex flex-col overflow-hidden">
          {/* Search bar */}
          <div className="p-3 border-b border-slate-800">
            <div className="flex items-center gap-2 bg-slate-800 rounded-lg px-3 py-2">
              <Search size={16} className="text-slate-500" />
              <input
                className="bg-transparent text-sm text-slate-200 w-full outline-none placeholder-slate-600"
                placeholder="Search songs in library..."
                value={searchQuery}
                onChange={(e) => searchSongs(e.target.value)}
              />
            </div>
          </div>

          {/* Results / Library */}
          <div className="flex-1 overflow-y-auto space-y-0.5 px-3 py-2">
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2 text-blue-400 text-xs font-bold uppercase tracking-wider">
                <ListMusic size={14} />
                <span>Library {searchQuery ? `(${searchResults.length})` : ''}</span>
              </div>
              <div className="flex items-center gap-2">
                <select 
                  className="bg-slate-800 text-[10px] text-slate-400 p-1 rounded border border-slate-700 outline-none uppercase font-bold"
                  value={sortBy}
                  onChange={(e) => {
                    const sort = e.target.value as any;
                    setSortBy(sort);
                    searchSongs(searchQuery, sort);
                  }}
                >
                  <option value="title">By Title</option>
                  <option value="artist">By Artist</option>
                  <option value="type">By Type</option>
                  <option value="recent">Recent</option>
                </select>
                <button onClick={clearLibrary} className="text-[10px] text-slate-500 hover:text-red-400 transition-colors uppercase font-bold" title="Clear Library DB">
                  Clear
                </button>
              </div>
            </div>
            {searchResults.length === 0 && searchQuery && (
              <div className="text-center py-6 text-sm text-slate-600 italic">No results</div>
            )}
            {searchResults.length === 0 && !searchQuery && (
              <div className="text-center py-6 text-sm text-slate-600 italic">
                {libraryReady ? 'Search your library above' : 'Click "Scan Folder" to build library'}
              </div>
            )}
            {searchResults.map((song: any) => (
              <div key={song.id}
                className="flex items-center justify-between p-2 rounded-lg bg-slate-800/30 hover:bg-slate-700/50 cursor-pointer transition-all group"
                onClick={() => enqueueSong(song.id, song.title)}
                title="Click to enqueue"
              >
                <div className="flex-1 min-w-0 mr-2 flex items-center gap-2">
                  <div className={`shrink-0 px-1.5 py-0.5 rounded text-[9px] uppercase tracking-wider font-bold border ${getEngineColor(song.file_type)}`}>
                    {song.file_type}
                  </div>
                  <div className="text-sm text-slate-200 truncate" title={song.title || song.file_path.split(/[\\/]/).pop()}>{song.title || song.file_path.split(/[\\/]/).pop()}</div>
                </div>
                <span className="text-[10px] text-slate-600 opacity-0 group-hover:opacity-100 transition-opacity">+ Queue</span>
              </div>
            ))}
          </div>

          <div className="h-px bg-slate-800" />

          {/* Queue */}
          <div className="h-48 overflow-y-auto px-3 py-2">
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2 text-blue-400 text-xs font-bold uppercase tracking-wider">
                <Users size={14} />
                <span>Queue ({queueItems.length})</span>
              </div>
              {queueItems.length > 0 && (
                <button onClick={clearQueue} className="text-[10px] text-slate-500 hover:text-red-400 transition-colors uppercase font-bold">
                  Clear All
                </button>
              )}
            </div>
            {queueItems.length === 0 ? (
              <div className="text-center py-4 text-sm text-slate-600 italic">Queue is empty</div>
            ) : (
              <div className="space-y-0.5">
                {queueItems.map((item: any, idx: number) => (
                  <div key={item.id}
                    className="flex items-center justify-between p-2 rounded-lg bg-slate-800/30 group hover:bg-slate-700/50 cursor-pointer"
                    onDoubleClick={() => playQueueItem(item)}
                    title="Double click to play now"
                  >
                    <div className="flex items-center gap-2 flex-1 min-w-0" onClick={() => playQueueItem(item)}>
                      <span className="text-[10px] text-slate-600 w-4 text-right">{idx + 1}</span>
                      <span className="text-sm text-slate-200 truncate">
                        {item.song?.title || `Song #${item.song_id}`}
                      </span>
                    </div>
                    <button
                      onClick={() => removeFromQueue(item.id)}
                      className="text-slate-600 hover:text-red-400 text-xs opacity-0 group-hover:opacity-100 transition-opacity"
                    >
                      ✕
                    </button>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Bottom Bar: Controls */}
      <div className="h-28 bg-slate-900 border-t border-slate-800 flex items-center px-6 gap-8">
        
        {/* Device Selectors */}
        <div className="flex flex-col gap-1 w-[280px] shrink-0">
          <label className="text-[10px] uppercase tracking-widest opacity-50 font-bold">Input Device</label>
          <select 
            className="bg-slate-800 text-xs p-1.5 rounded border border-slate-700 focus:outline-none focus:border-blue-500 w-full truncate"
            value={selectedInputDevice}
            onChange={(e) => setSelectedInputDevice(e.target.value)}
          >
            <option value="none">Disabled</option>
            {inputDevices.map((d, i) => (
              <option key={i} value={d.deviceId}>{d.label || `Microphone ${i + 1}`}</option>
            ))}
          </select>
          <label className="text-[10px] uppercase tracking-widest opacity-50 font-bold mt-0.5">Output Device</label>
          <select 
            className="bg-slate-800 text-xs p-1.5 rounded border border-slate-700 focus:outline-none focus:border-blue-500 w-full truncate"
            onChange={async (e) => {
              try {
                await invoke('set_output_device', { deviceName: e.target.value });
              } catch (err) {
                console.error('Erro ao trocar output device:', err);
              }
            }}
          >
            {outputDevices.map((d, i) => (
              <option key={i} value={d}>{d}</option>
            ))}
          </select>
        </div>

        {/* Transport Controls */}
        <div className="flex-1 flex flex-col items-center justify-center gap-1">
          <div className="flex items-center justify-center gap-6 w-full">
            <div className="text-3xl font-mono font-bold text-blue-400 w-20 text-center">
              {currentTime}
            </div>
            
            <div className="flex items-center gap-3">
              <button className="p-2 text-slate-400 hover:text-white transition-colors"><SkipBack size={24} /></button>
              <button 
                onClick={handlePlayToggle}
                className="p-4 bg-blue-600 rounded-full hover:bg-blue-500 transition-all transform hover:scale-110 active:scale-95 text-white shadow-lg shadow-blue-600/20"
              >
                {isPlaying ? <Pause size={32} fill="currentColor" /> : <Play size={32} fill="currentColor" />}
              </button>
              <button onClick={playNextInQueue} className="p-2 text-slate-400 hover:text-white transition-colors"><SkipForward size={24} /></button>
              <button 
                onClick={async () => { 
                  await invoke('stop_audio'); 
                  setIsPlaying(false); 
                  setCurrentTimeSec(0); 
                  setPlayMode('idle');
                }}
                className="p-2 text-slate-400 hover:text-red-500 transition-colors"
              ><StopCircle size={24} /></button>
            </div>

            <div className="flex items-center gap-3 w-48">
              <Volume2 size={20} className="text-slate-400" />
              <input
                type="range"
                min="0"
                max="100"
                defaultValue="80"
                onChange={async (e) => {
                  await invoke('set_volume', { volume: parseInt(e.target.value) / 100 });
                }}
                className="flex-1 accent-blue-500 h-1 bg-slate-700 rounded-lg appearance-none cursor-pointer"
              />
            </div>
          </div>

          {/* Seek / Progress Bar */}
          <div className="w-full max-w-2xl px-4">
            <input
              type="range"
              min="0"
              max={Math.max(
                midiMetadata?.total_seconds || ultrastarMetadata?.total_duration || 300,
                1
              )}
              value={currentTimeSec}
              onChange={async (e) => {
                const val = parseFloat(e.target.value);
                setCurrentTimeSec(val);
                try { await invoke('seek_to', { position: val }); } catch {}
              }}
              className="w-full accent-blue-500 h-1.5 bg-slate-700 rounded-lg appearance-none cursor-pointer"
            />
          </div>
        </div>

        {/* Action Buttons */}
        <div className="flex items-center gap-3">
          <div className="text-[10px] text-slate-600 text-right leading-tight">
            <div>{songCount} songs</div>
          </div>
          <button className="px-4 py-2 bg-slate-800 hover:bg-slate-700 rounded-lg text-xs font-bold transition-colors border border-slate-700">
            WEB BROWSER
          </button>
          <button className="px-4 py-2 bg-slate-800 hover:bg-slate-700 rounded-lg text-xs font-bold transition-colors border border-slate-700">
            SINGER'S LIST
          </button>
          <button className="px-4 py-2 bg-blue-600 hover:bg-blue-500 rounded-lg text-xs font-bold transition-colors shadow-lg shadow-blue-600/20">
            DUAL DISPLAY
          </button>
        </div>
      </div>
    </div>
  );
};

export default App;
