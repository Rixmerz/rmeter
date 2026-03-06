use std::path::PathBuf;
use std::process;
use std::time::Instant;

use clap::{Parser, Subcommand};
use tokio::sync::mpsc;

use rmeter_core::engine::{self, EngineConfig, EngineEvent, EngineStatus};
use rmeter_core::plan::io as plan_io;
use rmeter_core::results::{export, TestRunResult};

/// rmeter-cli — headless load testing from the command line
#[derive(Parser)]
#[command(name = "rmeter-cli", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a test plan from a .rmeter file
    Run {
        /// Path to the .rmeter plan file
        plan: PathBuf,

        /// Output format: summary (default), json, csv
        #[arg(short, long, default_value = "summary")]
        output: OutputFormat,

        /// Write results to a file instead of stdout
        #[arg(short = 'f', long)]
        output_file: Option<PathBuf>,

        /// Show live progress during execution
        #[arg(long, default_value = "true")]
        progress: bool,
    },
    /// Validate a .rmeter plan file without running it
    Validate {
        /// Path to the .rmeter plan file
        plan: PathBuf,
    },
}

#[derive(Clone, Debug)]
enum OutputFormat {
    Summary,
    Json,
    Csv,
    Html,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "summary" => Ok(Self::Summary),
            "json" => Ok(Self::Json),
            "csv" => Ok(Self::Csv),
            "html" => Ok(Self::Html),
            _ => Err(format!("Unknown output format: {s}. Expected: summary, json, csv, html")),
        }
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { plan, output, output_file, progress } => {
            run_test(plan, output, output_file, progress).await;
        }
        Commands::Validate { plan } => {
            validate_plan(plan).await;
        }
    }
}

async fn run_test(
    plan_path: PathBuf,
    output_format: OutputFormat,
    output_file: Option<PathBuf>,
    show_progress: bool,
) {
    // Load plan
    let plan = match plan_io::read_plan(&plan_path).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error loading plan from {}: {e}", plan_path.display());
            process::exit(1);
        }
    };

    let plan_name = plan.name.clone();
    let tg_count = plan.thread_groups.iter().filter(|tg| tg.enabled).count();
    let total_threads: u32 = plan.thread_groups.iter().filter(|tg| tg.enabled).map(|tg| tg.num_threads).sum();

    eprintln!("rmeter-cli — running \"{}\"", plan_name);
    eprintln!("  Thread groups: {tg_count}, Total threads: {total_threads}");
    eprintln!();

    // Set up engine
    let (tx, mut rx) = mpsc::channel(4096);
    let config = EngineConfig { plan, result_tx: tx };

    let start = Instant::now();
    let _handle = match engine::run_test(config).await {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Engine error: {e}");
            process::exit(1);
        }
    };

    // Collect events
    let mut results: Vec<rmeter_core::results::RequestResultEvent> = Vec::new();
    let time_series = Vec::new();
    let mut final_summary = None;

    while let Some(event) = rx.recv().await {
        match event {
            EngineEvent::Progress {
                completed_requests,
                total_errors,
                active_threads,
                elapsed_ms,
                current_rps,
                mean_ms,
                p95_ms,
                ..
            } => {
                if show_progress {
                    eprint!(
                        "\r  [{:.1}s] {} req | {} err | {} threads | {:.1} rps | mean {:.0}ms | p95 {}ms   ",
                        elapsed_ms as f64 / 1000.0,
                        completed_requests,
                        total_errors,
                        active_threads,
                        current_rps,
                        mean_ms,
                        p95_ms,
                    );
                }
            }
            EngineEvent::RequestResult(r) => {
                results.push(r);
            }
            EngineEvent::StatusChange { status } => {
                if status == EngineStatus::Completed || status == EngineStatus::Error {
                    if show_progress {
                        eprintln!();
                    }
                }
            }
            EngineEvent::Complete { summary } => {
                final_summary = Some(summary);
                break;
            }
        }
    }

    let elapsed = start.elapsed();

    let summary = match final_summary {
        Some(s) => s,
        None => {
            eprintln!("Test ended without producing a summary.");
            process::exit(1);
        }
    };

    // Build TestRunResult for export
    let run_result = TestRunResult {
        run_id: uuid::Uuid::new_v4(),
        summary: summary.clone(),
        time_series,
        request_results: results,
    };

    // Format output
    let output_content = match output_format {
        OutputFormat::Summary => format_summary(&summary, elapsed),
        OutputFormat::Json => match export::export_json(&run_result) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("Failed to serialize JSON: {e}");
                process::exit(1);
            }
        },
        OutputFormat::Csv => export::export_csv(&run_result),
        OutputFormat::Html => export::export_html(&run_result),
    };

    // Write output
    if let Some(path) = output_file {
        if let Err(e) = tokio::fs::write(&path, &output_content).await {
            eprintln!("Failed to write output to {}: {e}", path.display());
            process::exit(1);
        }
        eprintln!("Results written to {}", path.display());
    } else {
        println!("{output_content}");
    }

    // Exit with non-zero if there were errors
    if summary.failed_requests > 0 {
        process::exit(2);
    }
}

fn format_summary(s: &rmeter_core::results::TestSummary, elapsed: std::time::Duration) -> String {
    let total_errors = s.total_requests - s.successful_requests;
    let error_rate = if s.total_requests > 0 {
        total_errors as f64 / s.total_requests as f64
    } else {
        0.0
    };

    let mut out = String::new();
    out.push_str(&format!("=== Test Results: {} ===\n\n", s.plan_name));
    out.push_str(&format!("Total Requests:  {}\n", s.total_requests));
    out.push_str(&format!("Successful:      {}\n", s.successful_requests));
    out.push_str(&format!("Failed:          {}\n", s.failed_requests));
    out.push_str(&format!("Error Rate:      {:.2}%\n", error_rate * 100.0));
    out.push_str(&format!("Duration:        {:.2}s\n", elapsed.as_secs_f64()));
    out.push_str(&format!("Throughput:      {:.2} req/s\n\n", s.requests_per_second));
    out.push_str("Response Times:\n");
    out.push_str(&format!("  Mean:   {:.2} ms\n", s.mean_response_ms));
    out.push_str(&format!("  Min:    {} ms\n", s.min_response_ms));
    out.push_str(&format!("  Max:    {} ms\n", s.max_response_ms));
    out.push_str(&format!("  p50:    {} ms\n", s.p50_response_ms));
    out.push_str(&format!("  p95:    {} ms\n", s.p95_response_ms));
    out.push_str(&format!("  p99:    {} ms\n\n", s.p99_response_ms));
    out.push_str(&format!("Bytes Received:  {}\n", s.total_bytes_received));
    out
}

async fn validate_plan(plan_path: PathBuf) {
    match plan_io::read_plan(&plan_path).await {
        Ok(plan) => {
            let tg_count = plan.thread_groups.len();
            let req_count: usize = plan.thread_groups.iter().map(|tg| tg.requests.len()).sum();
            let elem_count: usize = plan.thread_groups.iter().map(|tg| tg.elements.len()).sum();
            println!("Plan \"{}\" is valid.", plan.name);
            println!("  Thread groups: {tg_count}");
            println!("  Requests: {req_count}");
            if elem_count > 0 {
                println!("  Elements: {elem_count}");
            }
            println!("  Variables: {}", plan.variables.len());
            println!("  CSV sources: {}", plan.csv_data_sources.len());
        }
        Err(e) => {
            eprintln!("Invalid plan at {}: {e}", plan_path.display());
            process::exit(1);
        }
    }
}
