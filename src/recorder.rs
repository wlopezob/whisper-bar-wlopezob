// src/recorder.rs

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};
use hound::{WavSpec, WavWriter};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const TARGET_SAMPLE_RATE: u32 = 16_000;
const OUTPUT_PATH: &str = "/tmp/whisperbar_recording.wav";
const SAMPLE_DRAIN_TIMEOUT: Duration = Duration::from_millis(250);

pub struct Recorder {
    stream: Option<Stream>,
    samples: Arc<Mutex<Vec<i16>>>,
    start_time: Option<Instant>,
    device_sample_rate: u32,
    device_channels: u16,
}

impl Recorder {
    pub fn new() -> Self {
        Recorder {
            stream: None,
            samples: Arc::new(Mutex::new(Vec::new())),
            start_time: None,
            device_sample_rate: TARGET_SAMPLE_RATE,
            device_channels: 1,
        }
    }

    /// Inicia la grabación desde el micrófono por defecto
    pub fn start(&mut self) -> Result<(), String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No se encontró dispositivo de entrada de audio")?;

        let supported_config = device
            .default_input_config()
            .map_err(|e| format!("Error obteniendo config de audio: {}", e))?;

        self.device_sample_rate = supported_config.sample_rate();
        self.device_channels = supported_config.channels();

        let samples = self.samples.clone();
        samples.lock().unwrap().clear();

        let channels = self.device_channels as usize;
        let sample_format = supported_config.sample_format();
        let config: cpal::StreamConfig = supported_config.into();

        let stream = match sample_format {
            SampleFormat::I16 => {
                let samples = samples.clone();
                device
                    .build_input_stream(
                        &config,
                        move |data: &[i16], _: &cpal::InputCallbackInfo| {
                            let mut buf = samples.lock().unwrap();
                            // Solo canal 0 → mono
                            for chunk in data.chunks(channels) {
                                buf.push(chunk[0]);
                            }
                        },
                        |err| eprintln!("Error stream de audio: {}", err),
                        None,
                    )
                    .map_err(|e| format!("Error creando stream i16: {}", e))?
            }
            SampleFormat::F32 => {
                let samples = samples.clone();
                device
                    .build_input_stream(
                        &config,
                        move |data: &[f32], _: &cpal::InputCallbackInfo| {
                            let mut buf = samples.lock().unwrap();
                            for chunk in data.chunks(channels) {
                                // Convertir f32 [-1.0, 1.0] → i16
                                let s = (chunk[0] * 32767.0).clamp(-32768.0, 32767.0) as i16;
                                buf.push(s);
                            }
                        },
                        |err| eprintln!("Error stream de audio: {}", err),
                        None,
                    )
                    .map_err(|e| format!("Error creando stream f32: {}", e))?
            }
            SampleFormat::U8 => {
                let samples = samples.clone();
                device
                    .build_input_stream(
                        &config,
                        move |data: &[u8], _: &cpal::InputCallbackInfo| {
                            let mut buf = samples.lock().unwrap();
                            for chunk in data.chunks(channels) {
                                // Convertir u8 [0, 255] → i16
                                let s = ((chunk[0] as i16 - 128) * 256) as i16;
                                buf.push(s);
                            }
                        },
                        |err| eprintln!("Error stream de audio: {}", err),
                        None,
                    )
                    .map_err(|e| format!("Error creando stream u8: {}", e))?
            }
            fmt => return Err(format!("Formato de audio no soportado: {:?}", fmt)),
        };

        stream.play().map_err(|e| format!("Error iniciando stream: {}", e))?;
        self.stream = Some(stream);
        self.start_time = Some(Instant::now());

        Ok(())
    }

    /// Detiene la grabación, escribe el WAV y retorna la duración en segundos
    pub fn stop(&mut self) -> Result<f64, String> {
        let duration = self
            .start_time
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0);

        // En algunos equipos, al soltar el hotkey puede haber una pequeña latencia
        // entre el fin de grabación y la llegada del primer callback de audio.
        if self.samples.lock().unwrap().is_empty() && self.stream.is_some() {
            let wait_start = Instant::now();
            while wait_start.elapsed() < SAMPLE_DRAIN_TIMEOUT {
                if !self.samples.lock().unwrap().is_empty() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }

        // Detener el stream (drop libera CoreAudio)
        self.stream = None;
        self.start_time = None;

        let samples = self.samples.lock().unwrap().clone();

        if samples.is_empty() {
            return Err(format!(
                "No se capturó audio (duración={:.2}s, rate={}Hz, canales={})",
                duration, self.device_sample_rate, self.device_channels
            ));
        }

        // Resample si el dispositivo no es 16kHz nativo
        let final_samples = if self.device_sample_rate != TARGET_SAMPLE_RATE {
            resample(&samples, self.device_sample_rate, TARGET_SAMPLE_RATE)
        } else {
            samples
        };

        // Escribir WAV con hound (16kHz, mono, 16-bit PCM — requerido por Whisper)
        let spec = WavSpec {
            channels: 1,
            sample_rate: TARGET_SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(OUTPUT_PATH, spec)
            .map_err(|e| format!("Error creando archivo WAV: {}", e))?;

        for sample in &final_samples {
            writer
                .write_sample(*sample)
                .map_err(|e| format!("Error escribiendo muestra WAV: {}", e))?;
        }

        writer
            .finalize()
            .map_err(|e| format!("Error finalizando WAV: {}", e))?;

        Ok(duration)
    }

    pub fn output_path(&self) -> &str {
        OUTPUT_PATH
    }
}

/// Resample lineal de source_rate → target_rate
/// Suficiente para whisper (no requiere filtro antialiasing de alta calidad)
fn resample(samples: &[i16], source_rate: u32, target_rate: u32) -> Vec<i16> {
    let ratio = source_rate as f64 / target_rate as f64;
    let output_len = (samples.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_pos = i as f64 * ratio;
        let idx = src_pos as usize;
        let frac = src_pos - idx as f64;

        let sample = if idx + 1 < samples.len() {
            // Interpolación lineal entre dos muestras adyacentes
            samples[idx] as f64 * (1.0 - frac) + samples[idx + 1] as f64 * frac
        } else if idx < samples.len() {
            samples[idx] as f64
        } else {
            0.0
        };

        output.push(sample.clamp(-32768.0, 32767.0) as i16);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample_same_rate() {
        let samples: Vec<i16> = vec![100, 200, 300, 400];
        let result = resample(&samples, 16_000, 16_000);
        assert_eq!(result, vec![100, 200, 300, 400]);
    }

    #[test]
    fn test_resample_downsample_44100_to_16000() {
        // 44100 → 16000: ratio ≈ 2.756, output debería ser más corto
        let samples: Vec<i16> = vec![0i16; 44100]; // 1 segundo a 44.1kHz
        let result = resample(&samples, 44_100, 16_000);
        // Esperamos ~16000 muestras (1 segundo a 16kHz)
        assert!(result.len() >= 15_900 && result.len() <= 16_100);
    }

    #[test]
    fn test_resample_empty() {
        let samples: Vec<i16> = vec![];
        let result = resample(&samples, 44_100, 16_000);
        assert!(result.is_empty());
    }

    #[test]
    fn test_recorder_new() {
        let recorder = Recorder::new();
        assert_eq!(recorder.output_path(), "/tmp/whisperbar_recording.wav");
        assert!(recorder.stream.is_none());
        assert!(recorder.start_time.is_none());
    }
}
