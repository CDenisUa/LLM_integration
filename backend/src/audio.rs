//! Audio assembly via ffmpeg (merge chunk WAVs into a chapter MP3).

// Core
use std::process::Command;

/// Whether an `ffmpeg` binary is available on PATH.
pub fn ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Concatenate WAV `inputs` (in order) into a single MP3 at `output`.
pub fn merge_wavs_to_mp3(inputs: &[String], output: &str) -> Result<(), String> {
    if inputs.is_empty() {
        return Err("no input audio".to_string());
    }

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y");
    for input in inputs {
        cmd.args(["-i", input]);
    }
    let filter = format!(
        "{}concat=n={}:v=0:a=1[out]",
        (0..inputs.len()).map(|i| format!("[{i}:a]")).collect::<String>(),
        inputs.len()
    );
    cmd.args(["-filter_complex", &filter, "-map", "[out]", output]);

    let result = cmd.output().map_err(|e| format!("ffmpeg failed to start: {e}"))?;
    if !result.status.success() {
        return Err(format!(
            "ffmpeg error: {}",
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Write a minimal 16-bit PCM mono WAV with `frames` silent samples.
    fn write_silence_wav(path: &str, frames: u32, sample_rate: u32) {
        let data_len = frames * 2;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
        bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
        bytes.extend_from_slice(&2u16.to_le_bytes()); // block align
        bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        bytes.extend(std::iter::repeat(0u8).take(data_len as usize));
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn merges_wavs_into_mp3_when_ffmpeg_present() {
        if !ffmpeg_available() {
            eprintln!("ffmpeg not available — skipping merge test");
            return;
        }
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("audio-merge-{unique}"));
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.wav").to_string_lossy().into_owned();
        let b = dir.join("b.wav").to_string_lossy().into_owned();
        let out = dir.join("chapter.mp3").to_string_lossy().into_owned();
        write_silence_wav(&a, 2400, 24000);
        write_silence_wav(&b, 2400, 24000);

        merge_wavs_to_mp3(&[a, b], &out).unwrap();

        let meta = std::fs::metadata(&out).unwrap();
        assert!(meta.len() > 0);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_empty_input() {
        assert!(merge_wavs_to_mp3(&[], "x.mp3").is_err());
    }
}
