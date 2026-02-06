use clap::{Parser, Subcommand};
use go_again::{parser, project, runner, storage, FailedTest};
use skim::prelude::*;
use std::io::{self, BufRead, Write};

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
        .prompt(String::from("Select tests to run> "))
        .build()
        .unwrap();

    let item_reader = SkimItemReader::default();
    let items_str = items.join("\n");
    let items = item_reader.of_bufread(std::io::Cursor::new(items_str));

    let selected = match Skim::run_with(options, Some(items)) {
        Ok(output) => output.selected_items,
        Err(_) => {
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

fn cmd_watch() -> io::Result<()> {
    let key = project::get_project_key()?;

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
            .prompt(String::from("Select tests to run (Ctrl-C to quit)> "))
            .build()
            .unwrap();

        let item_reader = SkimItemReader::default();
        let items_str = items.join("\n");
        let items = item_reader.of_bufread(std::io::Cursor::new(items_str));

        let selected = match Skim::run_with(options, Some(items)) {
            Ok(output) => output.selected_items,
            Err(_) => {
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
        println!("\nPress Enter to continue to selection...");

        // Wait for user to press Enter before returning to selection
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
    }
}

fn cmd_clear() -> io::Result<()> {
    let key = project::get_project_key()?;
    storage::clear_for_project(&key)?;
    println!("Cleared remembered tests for this project/branch.");
    Ok(())
}
