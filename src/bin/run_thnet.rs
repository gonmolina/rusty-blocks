use std::env;
use std::fs;
use std::process;
use bloques::thnet::{
    load_network, load_simulation_config, load_recorder_config, parse_signal
};
use bloques::thnet::solver::Solver;
use bloques::thnet::output::CsvRecorder;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Uso: {} <archivo_red.json>", args[0]);
        process::exit(1);
    }

    let json_path = &args[1];
    let content = match fs::read_to_string(json_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error leyendo archivo '{}': {}", json_path, e);
            process::exit(1);
        }
    };

    // 1. Cargar red
    let mut net = match load_network(&content) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("Error cargando la red: {}", e);
            process::exit(1);
        }
    };

    // 2. Cargar config de simulación
    let sim_config = match load_simulation_config(&content) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error cargando configuración de simulación: {}", e);
            process::exit(1);
        }
    };

    println!("--------------------------------------------------");
    println!("THNet Runner — Ejecución de red JSON");
    println!("Archivo: {}", json_path);
    println!("Nodos en la red: {}", net.nodes.len());
    println!("Tuberías en la red: {}", net.pipes.len());
    println!("Tanques abiertos: {}", net.open_tanks.len());
    println!("Tanques cerrados: {}", net.closed_tanks.len());
    println!("Tanques estratificados: {}", net.stratified_tanks.len());
    println!("Intercambiadores de calor: {}", net.heat_exchangers.len());
    println!("Simulando desde t=0.0s hasta t={:.2}s con dt={:.4}s", sim_config.t_final, sim_config.dt);
    println!("--------------------------------------------------");

    // 3. Inicializar y configurar el Solver
    let mut solver = Solver::new();
    solver.max_newton_iter = sim_config.max_newton_iter;
    solver.tol_flow = sim_config.tol_flow;

    // 4. Cargar opcionalmente el registrador CSV
    let mut recorder = match load_recorder_config(&content) {
        Ok(Some(rec_config)) => {
            let mut selectors = Vec::new();
            for sig in &rec_config.signals {
                match parse_signal(sig) {
                    Ok(sel) => selectors.push(sel),
                    Err(e) => {
                        eprintln!("Error interpretando señal '{}': {}", sig, e);
                        process::exit(1);
                    }
                }
            }
            println!("Registrador CSV habilitado en: {}", rec_config.file);
            println!("Señales a registrar: {:?}", rec_config.signals);
            match CsvRecorder::new(&rec_config.file, selectors) {
                Ok(rec) => Some(rec),
                Err(e) => {
                    eprintln!("Error inicializando CsvRecorder: {}", e);
                    process::exit(1);
                }
            }
        }
        Ok(None) => {
            println!("Aviso: No se configuró un registrador CSV ('recorder').");
            None
        }
        Err(e) => {
            eprintln!("Error leyendo configuración del registrador: {}", e);
            process::exit(1);
        }
    };

    // 5. Bucle de Simulación
    let steps = (sim_config.t_final / sim_config.dt).round() as usize;
    let print_every = (steps / 10).max(1);

    let start_time = std::time::Instant::now();

    // Registrar valor inicial t = 0.0
    if let Some(ref mut r) = recorder {
        if let Err(e) = r.record(0.0, &net) {
            eprintln!("Error de registro: {}", e);
        }
    }

    for step in 1..=steps {
        solver.step(&mut net, sim_config.dt);
        let t = solver.time;

        if let Some(ref mut r) = recorder {
            if let Err(e) = r.record(t, &net) {
                eprintln!("Error de registro: {}", e);
            }
        }

        if step % print_every == 0 || step == steps {
            println!("  Progreso: {:>3}% | t = {:>8.2} s", (step * 100) / steps, t);
        }
    }

    if let Some(ref mut r) = recorder {
        if let Err(e) = r.flush() {
            eprintln!("Error guardando CSV: {}", e);
        }
    }

    println!("--------------------------------------------------");
    println!("Ejecución completada con éxito en {:?}", start_time.elapsed());
    println!("--------------------------------------------------");
}
