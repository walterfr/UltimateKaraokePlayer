import React, { useState } from 'react';
import { 
  Play, Pause, SkipBack, SkipForward, StopCircle, 
  Volume2, Mic, Music, Search, Settings, 
  Monitor, Users, ListMusic, Equalizer
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/tauri';
import CdgCanvas from './components/CdgCanvas';

const App = () => {
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState('00:00');

  const handlePlayToggle = async () => {
    const nextState = !isPlaying;
    setIsPlaying(nextState);
    if (nextState) {
      try {
        // Dispara a leitura do arquivo no backend via Tauri IPC.
        // O Rust retornará eventos assíncronos no canal "cdg_batch" que o CdgCanvas já está escutando.
        await invoke('play_song', { path: 'dummy_music.mp3' });
      } catch (e) {
        console.error('Error starting playback:', e);
      }
    }
  };

  return (
    <div className="h-screen w-screen flex flex-col bg-karaoke-dark text-karaoke-text select-none overflow-hidden">
      {/* Main Content Area */}
      <div className="flex-1 flex overflow-hidden">
        
        {/* Left Panel: Settings & Mixing */}
        <div className="w-80 bg-karaoke-panel border-r border-slate-700 flex flex-col p-4 gap-6 overflow-y-auto">
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
              { label: 'Echo', icon: <Equalizer size={14}/> },
              { label: 'Reverb', icon: <Equalizer size={14}/> },
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
          {/* Background glow/animation placeholder */}
          <div className="absolute inset-0 bg-gradient-to-br from-blue-900/20 via-transparent to-purple-900/20 animate-pulse" />
          
          <div className="z-10 w-full h-full flex items-center justify-center p-8">
            <CdgCanvas />
          </div>

          {/* Overlays (only visible on hover) */}
          <div className="absolute top-4 right-4 opacity-0 group-hover:opacity-100 transition-opacity z-20">
            <button className="p-2 bg-slate-800/80 rounded-full hover:bg-blue-600 transition-colors">
              <Monitor size={20} />
            </button>
          </div>
        </div>

        {/* Right Panel: Library & Queue */}
        <div className="w-96 bg-karaoke-panel border-l border-slate-700 flex flex-col p-4 gap-6 overflow-hidden">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 text-blue-400 font-bold">
              <ListMusic size={20} />
              <h2>PLAYLIST</h2>
            </div>
            <button className="p-1.5 bg-slate-700 rounded hover:bg-slate-600 transition-colors">
              <Search size={16} />
            </button>
          </div>

          <div className="flex-1 overflow-y-auto space-y-1 pr-2 custom-scrollbar">
            {['14_Bis_Bola_de_meia.kar', '14_Bis_Criaturas_da_noite.kar', '14_Bis_Espanhola.kar', '14_Bis_Linda_juventude.kar'].map((song, i) => (
              <div 
                key={i} 
                className={`p-3 rounded-lg cursor-pointer transition-all flex justify-between items-center group
                  ${i === 0 ? 'bg-blue-600 text-white' : 'bg-slate-800/50 hover:bg-slate-700 text-slate-300'}`}
              >
                <span className="text-sm font-medium truncate mr-2">{song}</span>
                <span className="text-xs opacity-60">03:22</span>
              </div>
            ))}
          </div>

          <div className="h-px bg-slate-700 my-2" />

          <div className="flex items-center gap-2 text-blue-400 font-bold mb-2">
            <Users size={20} />
            <h2>LIVE PERFORMANCE</h2>
          </div>

          <div className="flex-1 overflow-y-auto space-y-1 pr-2 custom-scrollbar">
             {/* Queue goes here */}
             <div className="text-center py-8 text-sm opacity-40 italic">
               Queue is currently empty
             </div>
          </div>
        </div>
      </div>

      {/* Bottom Bar: Controls */}
      <div className="h-24 bg-slate-900 border-t border-slate-800 flex items-center px-6 gap-8">
        
        {/* Input Device Selector */}
        <div className="flex flex-col gap-1 min-w-[200px]">
          <label className="text-[10px] uppercase tracking-widest opacity-50 font-bold">Input Device</label>
          <select className="bg-slate-800 text-xs p-1.5 rounded border border-slate-700 focus:outline-none focus:border-blue-500">
            <option>Microphone (Steam Streaming Microphone)</option>
            <option>System Default</option>
          </select>
        </div>

        {/* Transport Controls */}
        <div className="flex-1 flex items-center justify-center gap-6">
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
            <button className="p-2 text-slate-400 hover:text-white transition-colors"><SkipForward size={24} /></button>
            <button className="p-2 text-slate-400 hover:text-red-500 transition-colors"><StopCircle size={24} /></button>
          </div>

          <div className="flex items-center gap-3 w-48">
            <Volume2 size={20} className="text-slate-400" />
            <input type="range" className="flex-1 accent-blue-500 h-1 bg-slate-700 rounded-lg appearance-none cursor-pointer" />
          </div>
        </div>

        {/* Action Buttons */}
        <div className="flex items-center gap-3">
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
