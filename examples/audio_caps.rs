// Diagnóstico: qué configuraciones de entrada expone el micro por defecto.
use cpal::traits::{DeviceTrait, HostTrait};

fn main() {
    let host = cpal::default_host();
    let device = host.default_input_device().expect("sin dispositivo de entrada");
    let desc = device.description().expect("sin descripción de dispositivo");
    println!("dispositivo: {}", desc.name());

    let d = device.default_input_config().expect("sin config por defecto");
    println!("default: {} Hz, {} ch, {:?}", d.sample_rate(), d.channels(), d.sample_format());

    println!("\nrangos soportados:");
    for c in device.supported_input_configs().expect("sin configs") {
        println!(
            "  {:>6}–{:<6} Hz  {} ch  {:?}",
            c.min_sample_rate(), c.max_sample_rate(), c.channels(), c.sample_format()
        );
    }
}
