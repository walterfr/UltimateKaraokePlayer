use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::os::raw::{c_void, c_int, c_long, c_ulong, c_float, c_double, c_char};
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;

pub type HSTREAM = c_ulong;
pub type HMUSIC = c_ulong;
pub type QWORD = u64;

// BASS Flags
const BASS_MUSIC_PRESCAN: c_ulong = 0x20000;
const BASS_POS_BYTE: c_ulong = 0;
const BASS_ATTRIB_VOL: c_ulong = 2;

// BASS API signatures
type BASS_Init_Fn = unsafe extern "system" fn(device: c_int, freq: c_ulong, flags: c_ulong, win: *mut c_void, clsid: *mut c_void) -> c_int;
type BASS_Free_Fn = unsafe extern "system" fn() -> c_int;
type BASS_MusicLoad_Fn = unsafe extern "system" fn(mem: c_int, file: *const c_void, offset: QWORD, length: c_ulong, flags: c_ulong, freq: c_ulong) -> HMUSIC;
type BASS_StreamCreateFile_Fn = unsafe extern "system" fn(mem: c_int, file: *const c_void, offset: QWORD, length: QWORD, flags: c_ulong) -> HSTREAM;
type BASS_ChannelPlay_Fn = unsafe extern "system" fn(handle: c_ulong, restart: c_int) -> c_int;
type BASS_ChannelStop_Fn = unsafe extern "system" fn(handle: c_ulong) -> c_int;
type BASS_ChannelPause_Fn = unsafe extern "system" fn(handle: c_ulong) -> c_int;
type BASS_ChannelSetAttribute_Fn = unsafe extern "system" fn(handle: c_ulong, attrib: c_ulong, value: c_float) -> c_int;
type BASS_ChannelSetPosition_Fn = unsafe extern "system" fn(handle: c_ulong, pos: QWORD, mode: c_ulong) -> c_int;
type BASS_ChannelBytes2Seconds_Fn = unsafe extern "system" fn(handle: c_ulong, pos: QWORD) -> c_double;
type BASS_ChannelSeconds2Bytes_Fn = unsafe extern "system" fn(handle: c_ulong, pos: c_double) -> QWORD;
type BASS_ChannelGetLength_Fn = unsafe extern "system" fn(handle: c_ulong, mode: c_ulong) -> QWORD;

struct BassApi {
    _lib: Library,
    init: BASS_Init_Fn,
    free: BASS_Free_Fn,
    music_load: BASS_MusicLoad_Fn,
    stream_create_file: BASS_StreamCreateFile_Fn,
    channel_play: BASS_ChannelPlay_Fn,
    channel_stop: BASS_ChannelStop_Fn,
    channel_pause: BASS_ChannelPause_Fn,
    channel_set_attribute: BASS_ChannelSetAttribute_Fn,
    channel_set_position: BASS_ChannelSetPosition_Fn,
    channel_bytes2seconds: BASS_ChannelBytes2Seconds_Fn,
    channel_seconds2bytes: BASS_ChannelSeconds2Bytes_Fn,
    channel_get_length: BASS_ChannelGetLength_Fn,
}

impl BassApi {
    unsafe fn load() -> Result<Self, String> {
        let mut lib = None;
        let mut last_err = String::new();

        let mut paths = vec![
            "bass.dll".to_string(),
            "src-tauri/bass.dll".to_string(),
            "../bass.dll".to_string(),
            "../../bass.dll".to_string(),
        ];

        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                paths.push(parent.join("bass.dll").to_string_lossy().to_string());
                if let Some(parent_parent) = parent.parent() {
                    paths.push(parent_parent.join("bass.dll").to_string_lossy().to_string());
                }
            }
        }
        
        if let Ok(current_dir) = std::env::current_dir() {
            paths.push(current_dir.join("src-tauri").join("bass.dll").to_string_lossy().to_string());
        }

        for path in paths {
            match Library::new(&path) {
                Ok(l) => {
                    lib = Some(l);
                    break;
                }
                Err(e) => {
                    last_err = format!("Failed to load BASS DLL at {}: {}", path, e);
                }
            }
        }

        let lib = lib.ok_or_else(|| last_err)?;
        
        let init: Symbol<BASS_Init_Fn> = lib.get(b"BASS_Init\0").map_err(|_| "Failed to load BASS_Init")?;
        let free: Symbol<BASS_Free_Fn> = lib.get(b"BASS_Free\0").map_err(|_| "Failed to load BASS_Free")?;
        let music_load: Symbol<BASS_MusicLoad_Fn> = lib.get(b"BASS_MusicLoad\0").map_err(|_| "Failed to load BASS_MusicLoad")?;
        let stream_create_file: Symbol<BASS_StreamCreateFile_Fn> = lib.get(b"BASS_StreamCreateFile\0").map_err(|_| "Failed to load BASS_StreamCreateFile")?;
        let channel_play: Symbol<BASS_ChannelPlay_Fn> = lib.get(b"BASS_ChannelPlay\0").map_err(|_| "Failed to load BASS_ChannelPlay")?;
        let channel_stop: Symbol<BASS_ChannelStop_Fn> = lib.get(b"BASS_ChannelStop\0").map_err(|_| "Failed to load BASS_ChannelStop")?;
        let channel_pause: Symbol<BASS_ChannelPause_Fn> = lib.get(b"BASS_ChannelPause\0").map_err(|_| "Failed to load BASS_ChannelPause")?;
        let channel_set_attribute: Symbol<BASS_ChannelSetAttribute_Fn> = lib.get(b"BASS_ChannelSetAttribute\0").map_err(|_| "Failed to load BASS_ChannelSetAttribute")?;
        let channel_set_position: Symbol<BASS_ChannelSetPosition_Fn> = lib.get(b"BASS_ChannelSetPosition\0").map_err(|_| "Failed to load BASS_ChannelSetPosition")?;
        let channel_bytes2seconds: Symbol<BASS_ChannelBytes2Seconds_Fn> = lib.get(b"BASS_ChannelBytes2Seconds\0").map_err(|_| "Failed to load BASS_ChannelBytes2Seconds")?;
        let channel_seconds2bytes: Symbol<BASS_ChannelSeconds2Bytes_Fn> = lib.get(b"BASS_ChannelSeconds2Bytes\0").map_err(|_| "Failed to load BASS_ChannelSeconds2Bytes")?;
        let channel_get_length: Symbol<BASS_ChannelGetLength_Fn> = lib.get(b"BASS_ChannelGetLength\0").map_err(|_| "Failed to load BASS_ChannelGetLength")?;

        Ok(Self {
            init: *init,
            free: *free,
            music_load: *music_load,
            stream_create_file: *stream_create_file,
            channel_play: *channel_play,
            channel_stop: *channel_stop,
            channel_pause: *channel_pause,
            channel_set_attribute: *channel_set_attribute,
            channel_set_position: *channel_set_position,
            channel_bytes2seconds: *channel_bytes2seconds,
            channel_seconds2bytes: *channel_seconds2bytes,
            channel_get_length: *channel_get_length,
            _lib: lib,
        })
    }
}

lazy_static! {
    static ref BASS_API: Mutex<Option<BassApi>> = Mutex::new(None);
}

pub struct BassEngine;

impl BassEngine {
    pub fn init() -> Result<(), String> {
        let mut api_guard = BASS_API.lock().unwrap();
        if api_guard.is_some() {
            return Ok(());
        }

        let api = unsafe { BassApi::load()? };
        
        let success = unsafe { (api.init)(-1, 44100, 0, std::ptr::null_mut(), std::ptr::null_mut()) };
        if success == 0 {
            // Error 14 is BASS_ERROR_ALREADY, which is fine
            println!("[BASS] Init returned 0, might already be initialized.");
        } else {
            println!("[BASS] Initialized successfully.");
        }

        *api_guard = Some(api);
        Ok(())
    }

    pub fn load_music(path: &str) -> Result<c_ulong, String> {
        let api_guard = BASS_API.lock().unwrap();
        let api = api_guard.as_ref().ok_or("BASS API not initialized")?;

        use std::os::windows::ffi::OsStrExt;
        let mut path_wide: Vec<u16> = std::ffi::OsStr::new(path).encode_wide().collect();
        path_wide.push(0);

        let flags: c_ulong = 0x80000000 | 0x20000 | 0x400 | 4; // BASS_UNICODE | BASS_MUSIC_PRESCAN | BASS_MUSIC_RAMPS | BASS_MUSIC_LOOP

        let mut handle = unsafe {
            (api.music_load)(
                0, // mem = false
                path_wide.as_ptr() as *const c_void,
                0,
                0,
                flags,
                0,
            )
        };

        if handle == 0 {
            // Se falhar como tracker, tenta como stream genérico (mk1, mp3, ogg, etc)
            let stream_flags: c_ulong = 0x80000000 | 0x20000 | 4; // BASS_UNICODE | BASS_STREAM_PRESCAN | BASS_SAMPLE_LOOP
            handle = unsafe {
                (api.stream_create_file)(
                    0,
                    path_wide.as_ptr() as *const c_void,
                    0,
                    0,
                    stream_flags,
                )
            };
        }

        if handle == 0 {
            let err_code = unsafe {
                let get_error: Symbol<unsafe extern "system" fn() -> c_int> = api._lib.get(b"BASS_ErrorGetCode\0").map_err(|_| "Could not find BASS_ErrorGetCode")?;
                get_error()
            };
            return Err(format!("Failed to load music with BASS (Error code: {})", err_code));
        }

        Ok(handle)
    }

    pub fn load_stream(path: &str) -> Result<c_ulong, String> {
        let api_guard = BASS_API.lock().unwrap();
        let api = api_guard.as_ref().ok_or("BASS API not initialized")?;

        let c_path = CString::new(path).map_err(|e| e.to_string())?;
        
        let handle = unsafe {
            (api.stream_create_file)(
                0, // mem = false
                c_path.as_ptr() as *const c_void,
                0,
                0,
                0 // flags
            )
        };

        if handle == 0 {
            Err("Failed to load stream with BASS".to_string())
        } else {
            Ok(handle)
        }
    }

    pub fn load_auto(path: &str) -> Result<c_ulong, String> {
        let mut actual_path = path.to_string();

        if path.to_lowercase().ends_with(".zip") {
            actual_path = Self::extract_zip_to_temp(path)?;
        }

        match Self::load_music(&actual_path) {
            Ok(h) => Ok(h),
            Err(_) => Self::load_stream(&actual_path)
        }
    }

    fn extract_zip_to_temp(zip_path: &str) -> Result<String, String> {
        let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
        
        let valid_exts = ["mod", "s3m", "xm", "it", "st3", "mk1", "kara"];
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
            let outpath = match file.enclosed_name() {
                Some(path) => path.to_owned(),
                None => continue,
            };
            
            if let Some(ext) = outpath.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if valid_exts.contains(&ext_str.as_str()) {
                    let mut temp_path = std::env::temp_dir();
                    temp_path.push(outpath.file_name().unwrap_or(std::ffi::OsStr::new("extracted_tracker.tmp")));
                    
                    let mut outfile = std::fs::File::create(&temp_path).map_err(|e| e.to_string())?;
                    std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
                    
                    return Ok(temp_path.to_string_lossy().to_string());
                }
            }
        }
        
        Err("No supported tracker or legacy file found in zip".to_string())
    }

    pub fn play(handle: c_ulong) -> Result<(), String> {
        let api_guard = BASS_API.lock().unwrap();
        let api = api_guard.as_ref().ok_or("BASS API not initialized")?;
        
        let success = unsafe { (api.channel_play)(handle, 0) };
        if success == 0 {
            Err("Failed to play channel".to_string())
        } else {
            Ok(())
        }
    }

    pub fn stop(handle: c_ulong) -> Result<(), String> {
        let api_guard = BASS_API.lock().unwrap();
        let api = api_guard.as_ref().ok_or("BASS API not initialized")?;
        
        unsafe { (api.channel_stop)(handle) };
        Ok(())
    }

    pub fn pause(handle: c_ulong) -> Result<(), String> {
        let api_guard = BASS_API.lock().unwrap();
        let api = api_guard.as_ref().ok_or("BASS API not initialized")?;
        
        unsafe { (api.channel_pause)(handle) };
        Ok(())
    }

    pub fn set_volume(handle: c_ulong, volume: f32) -> Result<(), String> {
        let api_guard = BASS_API.lock().unwrap();
        let api = api_guard.as_ref().ok_or("BASS API not initialized")?;
        
        unsafe { (api.channel_set_attribute)(handle, BASS_ATTRIB_VOL, volume) };
        Ok(())
    }

    pub fn seek(handle: c_ulong, seconds: f64) -> Result<(), String> {
        let api_guard = BASS_API.lock().unwrap();
        let api = api_guard.as_ref().ok_or("BASS API not initialized")?;
        
        let bytes = unsafe { (api.channel_seconds2bytes)(handle, seconds) };
        unsafe { (api.channel_set_position)(handle, bytes, BASS_POS_BYTE) };
        Ok(())
    }

    pub fn get_duration(handle: c_ulong) -> f64 {
        let api_guard = BASS_API.lock().unwrap();
        if let Some(api) = api_guard.as_ref() {
            let bytes = unsafe { (api.channel_get_length)(handle, BASS_POS_BYTE) };
            if bytes != u64::MAX { // BASS_ERROR_NOTAVAIL
                return unsafe { (api.channel_bytes2seconds)(handle, bytes) };
            }
        }
        0.0
    }
}
