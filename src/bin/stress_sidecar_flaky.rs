use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const DEFAULT_ITERATIONS: u32 = 12;
const DEFAULT_TEST_NAME: &str =
    "sidecar::tests::test_generate_sidecar_existing_sidecar_logs_skip_with_path";
const DEFAULT_LOG_DIR: &str = "target/stress-logs/sidecar-flaky";

#[derive(Debug)]
struct Config {
    iterations: u32,
    test_name: String,
    log_dir: PathBuf,
}

fn main() -> Result<(), String> {
    let config = parse_config(env::args().skip(1).collect())?;
    fs::create_dir_all(&config.log_dir).map_err(|e| {
        format!(
            "failed to create log directory '{}': {e}",
            config.log_dir.display()
        )
    })?;

    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut summary = String::new();
    summary.push_str(&format!("test_name={}\n", config.test_name));
    summary.push_str(&format!("iterations={}\n", config.iterations));
    summary.push_str(&format!("log_dir={}\n\n", config.log_dir.display()));

    println!(
        "Running stress lane for '{}' ({} iterations)",
        config.test_name, config.iterations
    );

    for iteration in 1..=config.iterations {
        let started = Instant::now();
        let output = Command::new(&cargo)
            .args([
                "test",
                "--lib",
                &config.test_name,
                "--",
                "--exact",
                "--color",
                "never",
            ])
            .output()
            .map_err(|e| format!("failed to run cargo test on iteration {iteration}: {e}"))?;
        let elapsed_ms = started.elapsed().as_millis();

        let status = output.status.code().unwrap_or(-1);
        let log_path = config.log_dir.join(format!("iteration_{iteration:03}.log"));
        let log_contents = format!(
            "iteration={iteration}\nstatus={status}\nelapsed_ms={elapsed_ms}\n\nstdout:\n{}\n\nstderr:\n{}\n",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        fs::write(&log_path, log_contents).map_err(|e| {
            format!(
                "failed to write iteration log '{}': {e}",
                log_path.display()
            )
        })?;

        summary.push_str(&format!(
            "iteration={iteration:03} status={status} elapsed_ms={elapsed_ms} log={}\n",
            log_path.display()
        ));

        if !output.status.success() {
            let marker_path = config.log_dir.join("failed_iteration.txt");
            fs::write(
                &marker_path,
                format!("failed_iteration={iteration}\nlog={}\n", log_path.display()),
            )
            .map_err(|e| {
                format!(
                    "failed to write failure marker '{}': {e}",
                    marker_path.display()
                )
            })?;
            let summary_path = config.log_dir.join("summary.txt");
            fs::write(&summary_path, &summary).map_err(|e| {
                format!("failed to write summary '{}': {e}", summary_path.display())
            })?;

            eprintln!(
                "Stress lane failed on iteration {iteration}. See '{}'.",
                log_path.display()
            );
            return Err("stress lane failed".to_string());
        }
    }

    let summary_path = config.log_dir.join("summary.txt");
    fs::write(&summary_path, summary)
        .map_err(|e| format!("failed to write summary '{}': {e}", summary_path.display()))?;
    println!(
        "Stress lane passed ({} iterations). Logs at '{}'.",
        config.iterations,
        config.log_dir.display()
    );
    Ok(())
}

fn parse_config(args: Vec<String>) -> Result<Config, String> {
    let mut iterations = env::var("STRESS_ITERATIONS")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(DEFAULT_ITERATIONS);
    let mut test_name = DEFAULT_TEST_NAME.to_string();
    let mut log_dir = PathBuf::from(DEFAULT_LOG_DIR);

    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--iterations" | "-n" => {
                idx += 1;
                let value = args
                    .get(idx)
                    .ok_or_else(|| "missing value for --iterations".to_string())?;
                iterations = value
                    .parse::<u32>()
                    .map_err(|_| format!("invalid --iterations value '{value}'"))?;
            }
            "--test-name" => {
                idx += 1;
                let value = args
                    .get(idx)
                    .ok_or_else(|| "missing value for --test-name".to_string())?;
                test_name = value.clone();
            }
            "--log-dir" => {
                idx += 1;
                let value = args
                    .get(idx)
                    .ok_or_else(|| "missing value for --log-dir".to_string())?;
                log_dir = PathBuf::from(value);
            }
            "--help" | "-h" => {
                print_usage(Path::new(DEFAULT_LOG_DIR));
                std::process::exit(0);
            }
            other => {
                return Err(format!("unknown argument '{other}'"));
            }
        }
        idx += 1;
    }

    if iterations == 0 {
        return Err("--iterations must be >= 1".to_string());
    }

    Ok(Config {
        iterations,
        test_name,
        log_dir,
    })
}

fn print_usage(default_log_dir: &Path) {
    println!("stress-sidecar-flaky");
    println!();
    println!("Usage:");
    println!("  cargo run --bin stress-sidecar-flaky -- [OPTIONS]");
    println!();
    println!("Options:");
    println!("  -n, --iterations <N>  Number of repetitions (default: {DEFAULT_ITERATIONS})");
    println!("      --test-name <S>   Test name to run (default: {DEFAULT_TEST_NAME})");
    println!(
        "      --log-dir <PATH>  Directory for iteration logs (default: {})",
        default_log_dir.display()
    );
    println!("  -h, --help            Show this help");
    println!();
    println!("Environment:");
    println!("  STRESS_ITERATIONS overrides default iteration count.");
}
