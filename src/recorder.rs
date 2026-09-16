// src/recorder.rs

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};
use hound::{WavSpec, WavWriter};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const TARGET_SAMPLE_RATE: u32 = 16_000;
const OUTPUT_PATH: &str = "/tmp/whisperbar_recording.wav";
const SAMPLE_DRAIN_TIMEOUT: Duration = Duration::from_millis(250);

/// Coeficientes del FIR anti-aliasing. Impar para que el retardo de grupo sea
/// un número entero de muestras y el filtro no desplace el audio en el tiempo.
const ANTIALIAS_TAPS: usize = 101;

/// Corte del filtro como fracción de la Nyquist del destino. 0.9 deja banda de
/// transición suficiente para que en la Nyquist exacta ya esté atenuado.
const ANTIALIAS_CUTOFF: f64 = 0.9;

/// Si esta variable de entorno está definida, cada grabación vuelca además el
/// audio crudo del micro, sin resamplear ni filtrar. Es un diagnóstico temporal
/// para medir cuánta energía hay por encima de la Nyquist del destino y poder
/// comparar transcripciones con y sin filtro sobre el mismo audio real.
///
///   open --env WHISPERBAR_DUMP_RAW=1 /Applications/whisperwlopezob.app
const DUMP_RAW_ENV: &str = "WHISPERBAR_DUMP_RAW";

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

        if std::env::var_os(DUMP_RAW_ENV).is_some() {
            dump_raw(&samples, self.device_sample_rate);
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

/// Escribe el audio tal y como lo entregó el micro, sin resamplear ni filtrar.
///
/// Cada grabación va a su propio archivo para poder acumular varias muestras:
/// una sola utterance no da para concluir nada sobre acierto de transcripción.
/// Nunca falla de forma ruidosa — es diagnóstico, no debe romper el dictado.
fn dump_raw(samples: &[i16], sample_rate: u32) {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let path = format!("/tmp/whisperbar-raw-{}.wav", stamp);

    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let writer = match WavWriter::create(&path, spec) {
        Ok(w) => w,
        Err(e) => {
            log::error!("Volcado crudo: no se pudo crear {}: {}", path, e);
            return;
        }
    };

    let mut writer = writer;
    for sample in samples {
        if let Err(e) = writer.write_sample(*sample) {
            log::error!("Volcado crudo: error escribiendo: {}", e);
            return;
        }
    }

    match writer.finalize() {
        Ok(()) => log::info!(
            "Volcado crudo: {} ({} Hz, {:.1}s)",
            path, sample_rate, samples.len() as f64 / sample_rate as f64
        ),
        Err(e) => log::error!("Volcado crudo: error cerrando: {}", e),
    }
}

/// Resample de source_rate → target_rate, con filtro anti-aliasing al bajar.
fn resample(samples: &[i16], source_rate: u32, target_rate: u32) -> Vec<i16> {
    // Al bajar la frecuencia de muestreo, todo lo que supera la Nyquist del
    // destino no se pierde: se refleja hacia abajo dentro de la banda útil. De
    // 48 kHz a 16 kHz, un sonido de 13 kHz reaparece como 3 kHz, justo sobre
    // los formantes que distinguen las vocales. Hay que quitarlo ANTES de
    // descartar muestras: después el alias es indistinguible de la señal real.
    let filtered: Vec<i16>;
    let samples = if source_rate > target_rate {
        filtered = lowpass(samples, source_rate, target_rate);
        &filtered
    } else {
        samples
    };

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

/// Aplica un paso-bajo FIR que elimina lo que quedaría por encima de la
/// Nyquist del destino. Convolución centrada: fuera de los extremos se asume
/// silencio, así que el audio no se desplaza respecto al original.
fn lowpass(samples: &[i16], source_rate: u32, target_rate: u32) -> Vec<i16> {
    // Corte expresado como fracción de la frecuencia de muestreo de origen.
    let cutoff = (target_rate as f64 / 2.0) * ANTIALIAS_CUTOFF / source_rate as f64;
    let kernel = antialias_kernel(cutoff);
    let half = kernel.len() / 2;

    let mut out = Vec::with_capacity(samples.len());
    for i in 0..samples.len() {
        let mut acc = 0.0;
        for (k, coef) in kernel.iter().enumerate() {
            let idx = i as isize + k as isize - half as isize;
            if idx >= 0 && (idx as usize) < samples.len() {
                acc += samples[idx as usize] as f64 * coef;
            }
        }
        out.push(acc.clamp(-32768.0, 32767.0) as i16);
    }
    out
}

/// Sinc enventanado con Hamming, normalizado a ganancia unidad en continua.
///
/// El sinc puro es la respuesta ideal pero infinita; truncarlo de golpe genera
/// ondulaciones (Gibbs). La ventana de Hamming suaviza los extremos y baja los
/// lóbulos laterales a ~-53 dB, suficiente para voz.
fn antialias_kernel(cutoff: f64) -> Vec<f64> {
    use std::f64::consts::PI;

    let n = ANTIALIAS_TAPS;
    let center = (n - 1) as f64 / 2.0;
    let mut kernel = Vec::with_capacity(n);
    let mut sum = 0.0;

    for i in 0..n {
        let x = i as f64 - center;
        let sinc = if x.abs() < 1e-9 {
            2.0 * cutoff
        } else {
            (2.0 * PI * cutoff * x).sin() / (PI * x)
        };
        let window = 0.54 - 0.46 * (2.0 * PI * i as f64 / (n - 1) as f64).cos();
        let coef = sinc * window;
        kernel.push(coef);
        sum += coef;
    }

    // Sin normalizar, el filtro alteraría el volumen del audio.
    for coef in kernel.iter_mut() {
        *coef /= sum;
    }
    kernel
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

    /// Amplitud de una frecuencia concreta dentro de una señal (Goertzel).
    fn amplitud_de(samples: &[i16], freq: f64, rate: f64) -> f64 {
        use std::f64::consts::PI;
        let (mut re, mut im) = (0.0, 0.0);
        for (n, &s) in samples.iter().enumerate() {
            let phase = 2.0 * PI * freq * n as f64 / rate;
            re += s as f64 * phase.cos();
            im += s as f64 * phase.sin();
        }
        (re * re + im * im).sqrt() / samples.len() as f64
    }

    fn tono(freq: f64, rate: f64, muestras: usize) -> Vec<i16> {
        use std::f64::consts::PI;
        (0..muestras)
            .map(|n| ((2.0 * PI * freq * n as f64 / rate).sin() * 16_000.0) as i16)
            .collect()
    }

    #[test]
    fn test_resample_elimina_el_alias_de_13khz() {
        // 13 kHz supera la Nyquist del destino (8 kHz). Sin filtro se pliega y
        // reaparece como 16000 - 13000 = 3000 Hz, encima de los formantes.
        let entrada = tono(13_000.0, 48_000.0, 48_000);
        let salida = resample(&entrada, 48_000, 16_000);

        let alias = amplitud_de(&salida, 3_000.0, 16_000.0);
        let referencia = amplitud_de(&tono(3_000.0, 16_000.0, 16_000), 3_000.0, 16_000.0);

        assert!(
            alias < referencia * 0.05,
            "el alias de 3 kHz sigue presente: {:.1} vs referencia {:.1}",
            alias, referencia
        );
    }

    #[test]
    fn test_resample_conserva_la_banda_de_voz() {
        // 1 kHz está muy dentro de la banda útil: debe pasar casi intacto.
        let entrada = tono(1_000.0, 48_000.0, 48_000);
        let salida = resample(&entrada, 48_000, 16_000);

        let antes = amplitud_de(&entrada, 1_000.0, 48_000.0);
        let despues = amplitud_de(&salida, 1_000.0, 16_000.0);

        assert!(
            despues > antes * 0.9,
            "el filtro se está comiendo la voz: {:.1} → {:.1}",
            antes, despues
        );
    }

    #[test]
    fn test_kernel_tiene_ganancia_unidad() {
        // Si los coeficientes no suman 1, el filtro cambia el volumen.
        let suma: f64 = antialias_kernel(0.15).iter().sum();
        assert!((suma - 1.0).abs() < 1e-9, "suma de coeficientes = {}", suma);
    }

    #[test]
    fn test_recorder_new() {
        let recorder = Recorder::new();
        assert_eq!(recorder.output_path(), "/tmp/whisperbar_recording.wav");
        assert!(recorder.stream.is_none());
        assert!(recorder.start_time.is_none());
    }
}
