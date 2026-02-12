use clap::{Parser, Subcommand};
use go_again::{parser, project, runner, storage, FailedTest};
use notify::{RecursiveMode, Watcher};
use skim::prelude::*;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::sync::mpsc;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "go-again")]
#[command(about = "Remember and re-run failing Go tests", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Read go test output from stdin and remember failing tests
    Remember,

    /// Re-run remembered failing tests
    Run {
        /// Remove passing tests from the remembered list
        #[arg(long)]
        update: bool,
    },

    /// List remembered failing tests
    List,

    /// Interactively select which tests to re-run
    Select,

    /// Interactively select and re-run tests in a loop
    Watch,

    /// Clear all remembered tests for current project
    Clear,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Remember => cmd_remember(),
        Commands::Run { update } => cmd_run(update),
        Commands::List => cmd_list(),
        Commands::Select => cmd_select(),
        Commands::Watch => cmd_watch(),
        Commands::Clear => cmd_clear(),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn cmd_remember() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut lines: Vec<String> = Vec::new();

    // Stream each line to stdout as we receive it
    for line in stdin.lock().lines() {
        let line = line?;
        writeln!(stdout, "{}", line)?;
        stdout.flush()?;
        lines.push(line);
    }

    let input = lines.join("\n");
    let failed_tests = parser::parse_go_test_output(input.as_bytes());

    let key = project::get_project_key()?;

    if failed_tests.is_empty() {
        println!("\n[go-again] All tests passed! Clearing remembered failures.");
        storage::clear_for_project(&key)?;
    } else {
        println!(
            "\n[go-again] Remembered {} failing test(s):",
            failed_tests.len()
        );
        for test in &failed_tests {
            println!("  {} {}", test.package, test.name);
        }
        storage::set_for_project(&key, failed_tests)?;
    }

    Ok(())
}

fn cmd_run(update: bool) -> io::Result<()> {
    let key = project::get_project_key()?;
    let tests = storage::get_for_project(&key)?;

    if tests.is_empty() {
        println!("No failing tests remembered for this project/branch.");
        return Ok(());
    }

    println!("Running {} remembered test(s)...\n", tests.len());

    let results = runner::run_tests(&tests);

    let mut still_failing: Vec<FailedTest> = Vec::new();
    let mut passed_count = 0;
    let mut failed_count = 0;

    for result in results {
        let status = if result.passed {
            passed_count += 1;
            "PASS"
        } else {
            failed_count += 1;
            still_failing.push(result.test.clone());
            "FAIL"
        };

        println!("[{}] {} {}", status, result.test.package, result.test.name);

        if !result.passed {
            // Show output for failed tests
            for line in result.output.lines().take(20) {
                println!("  {}", line);
            }
            if result.output.lines().count() > 20 {
                println!("  ... (output truncated)");
            }
            println!();
        }
    }

    println!(
        "\nResults: {} passed, {} failed",
        passed_count, failed_count
    );

    if update {
        if still_failing.is_empty() {
            println!("\n[go-again] All tests now pass! Clearing remembered failures.");
            storage::clear_for_project(&key)?;
        } else {
            println!(
                "\n[go-again] Updated: {} test(s) still failing.",
                still_failing.len()
            );
            storage::set_for_project(&key, still_failing)?;
        }
    }

    Ok(())
}

fn cmd_list() -> io::Result<()> {
    let key = project::get_project_key()?;
    let tests = storage::get_for_project(&key)?;

    if tests.is_empty() {
        println!("No failing tests remembered for this project/branch.");
        return Ok(());
    }

    for test in tests {
        println!("{} {}", test.package, test.name);
    }

    Ok(())
}

fn cmd_select() -> io::Result<()> {
    let key = project::get_project_key()?;
    let tests = storage::get_for_project(&key)?;

    if tests.is_empty() {
        println!("No failing tests remembered for this project/branch.");
        return Ok(());
    }

    // Build items for skim
    let items: Vec<String> = tests
        .iter()
        .map(|t| format!("{} {}", t.package, t.name))
        .collect();

    let options = SkimOptionsBuilder::default()
        .multi(true)
        .prompt(Some("Select tests to run> "))
        .build()
        .unwrap();

    let item_reader = SkimItemReader::default();
    let items_str = items.join("\n");
    let items = item_reader.of_bufread(std::io::Cursor::new(items_str));

    let selected = match Skim::run_with(&options, Some(items)) {
        Some(output) if !output.is_abort => output.selected_items,
        _ => {
            println!("Selection cancelled.");
            return Ok(());
        }
    };

    if selected.is_empty() {
        println!("No tests selected.");
        return Ok(());
    }

    // Parse selected items back to FailedTest
    let selected_tests: Vec<FailedTest> = selected
        .iter()
        .filter_map(|item| {
            let text = item.output().to_string();
            let mut parts = text.splitn(2, ' ');
            let package = parts.next()?.to_string();
            let name = parts.next()?.to_string();
            Some(FailedTest { package, name })
        })
        .collect();

    println!("\nRunning {} selected test(s)...\n", selected_tests.len());

    let results = runner::run_tests(&selected_tests);

    for result in &results {
        let status = if result.passed { "PASS" } else { "FAIL" };
        println!("[{}] {} {}", status, result.test.package, result.test.name);

        if !result.passed {
            for line in result.output.lines().take(20) {
                println!("  {}", line);
            }
            if result.output.lines().count() > 20 {
                println!("  ... (output truncated)");
            }
            println!();
        }
    }

    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.len() - passed;
    println!("\nResults: {} passed, {} failed", passed, failed);

    Ok(())
}

enum WatchEvent {
    FileChanged,
    SelectTests,
}

fn run_and_print(tests: &[FailedTest]) {
    println!("\nRunning {} selected test(s)...\n", tests.len());

    let results = runner::run_tests(tests);

    for result in &results {
        let status = if result.passed { "PASS" } else { "FAIL" };
        println!("[{}] {} {}", status, result.test.package, result.test.name);

        if !result.passed {
            for line in result.output.lines().take(20) {
                println!("  {}", line);
            }
            if result.output.lines().count() > 20 {
                println!("  ... (output truncated)");
            }
            println!();
        }
    }

    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.len() - passed;
    println!("\nResults: {} passed, {} failed", passed, failed);
}

fn cmd_watch() -> io::Result<()> {
    let key = project::get_project_key()?;
    let git_root = project::get_git_root()?;

    loop {
        let tests = storage::get_for_project(&key)?;

        if tests.is_empty() {
            println!("No failing tests remembered for this project/branch.");
            return Ok(());
        }

        // Build items for skim
        let items: Vec<String> = tests
            .iter()
            .map(|t| format!("{} {}", t.package, t.name))
            .collect();

        let options = SkimOptionsBuilder::default()
            .multi(true)
            .prompt(Some("Select tests to run (Ctrl-C to quit)> "))
            .build()
            .unwrap();

        let item_reader = SkimItemReader::default();
        let items_str = items.join("\n");
        let items = item_reader.of_bufread(std::io::Cursor::new(items_str));

        let selected = match Skim::run_with(&options, Some(items)) {
            Some(output) if !output.is_abort => output.selected_items,
            _ => {
                println!("Watch mode ended.");
                return Ok(());
            }
        };

        if selected.is_empty() {
            println!("No tests selected. Select tests or press Ctrl-C to quit.");
            continue;
        }

        // Parse selected items back to FailedTest
        let selected_tests: Vec<FailedTest> = selected
            .iter()
            .filter_map(|item| {
                let text = item.output().to_string();
                let mut parts = text.splitn(2, ' ');
                let package = parts.next()?.to_string();
                let name = parts.next()?.to_string();
                Some(FailedTest { package, name })
            })
            .collect();

        // Initial run
        run_and_print(&selected_tests);

        // Set up channel for watch events
        let (tx, rx) = mpsc::channel();

        // Start file watcher
        let tx_watcher = tx.clone();
        let mut watcher =
            notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    let dominated_by_git = event
                        .paths
                        .iter()
                        .all(|p| p.components().any(|c| c.as_os_str() == ".git"));
                    if !dominated_by_git {
                        let _ = tx_watcher.send(WatchEvent::FileChanged);
                    }
                }
            })
            .map_err(|e| io::Error::other(format!("Failed to create file watcher: {e}")))?;

        watcher
            .watch(Path::new(&git_root), RecursiveMode::Recursive)
            .map_err(|e| io::Error::other(format!("Failed to watch directory: {e}")))?;

        // Spawn stdin reader thread
        let tx_stdin = tx.clone();
        std::thread::spawn(move || {
            let stdin = io::stdin();
            let mut line = String::new();
            loop {
                line.clear();
                if stdin.lock().read_line(&mut line).is_err() || line.is_empty() {
                    break;
                }
                let _ = tx_stdin.send(WatchEvent::SelectTests);
            }
        });

        println!("\nWatching for changes... (Enter to re-select, Ctrl-C to quit)");

        let mut last_run = Instant::now();

        // Event loop
        loop {
            match rx.recv() {
                Ok(WatchEvent::FileChanged) => {
                    if last_run.elapsed().as_millis() < 500 {
                        continue;
                    }
                    run_and_print(&selected_tests);
                    last_run = Instant::now();
                    println!(
                        "\nWatching for changes... (Enter to re-select, Ctrl-C to quit)"
                    );
                }
                Ok(WatchEvent::SelectTests) => {
                    // Drop watcher by breaking; outer loop will re-select
                    break;
                }
                Err(_) => {
                    // Channel closed, exit
                    return Ok(());
                }
            }
        }
    }
}

fn cmd_clear() -> io::Result<()> {
    let key = project::get_project_key()?;
    storage::clear_for_project(&key)?;
    println!("Cleared remembered tests for this project/branch.");
    Ok(())
}
