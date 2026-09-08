mod lint;
mod parser;

use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

use lint::{check_routes, Finding, Severity};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    let mut json_output = false;
    let mut path: Option<String> = None;

    for arg in args {
        match arg.as_str() {
            "--json" => json_output = true,
            "-h" | "--help" => {
                print_usage();
                return ExitCode::SUCCESS;
            }
            other => {
                if path.is_some() {
                    eprintln!("routelint: unexpected argument \"{}\"", other);
                    print_usage();
                    return ExitCode::FAILURE;
                }
                path = Some(other.to_string());
            }
        }
    }

    let path = match path {
        Some(p) => p,
        None => {
            print_usage();
            return ExitCode::FAILURE;
        }
    };

    let contents = if path == "-" {
        let mut buf = String::new();
        match io::stdin().read_to_string(&mut buf) {
            Ok(_) => buf,
            Err(err) => {
                eprintln!("routelint: cannot read stdin: {}", err);
                return ExitCode::FAILURE;
            }
        }
    } else {
        match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(err) => {
                eprintln!("routelint: cannot read {}: {}", path, err);
                return ExitCode::FAILURE;
            }
        }
    };

    // "<stdin>" reads better than "-" in output meant for a person or a JSON consumer.
    let label = if path == "-" { "<stdin>".to_string() } else { path };

    let (routes, unparsable) = parser::parse(&contents);
    let mut findings = check_routes(&routes);

    for line in &unparsable {
        findings.push(Finding {
            line: *line,
            severity: Severity::Warning,
            rule: "unparsable-line",
            message: "line does not match \"METHOD /path -> handler\"".to_string(),
        });
    }
    findings.sort_by_key(|f| f.line);

    if json_output {
        print_json(&label, &findings);
    } else {
        print_human(&label, &findings);
    }

    let has_errors = findings.iter().any(|f| f.severity == Severity::Error);
    if has_errors {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn print_usage() {
    eprintln!("usage: routelint <routes-file> [--json]");
    eprintln!("       routelint - [--json]   (read routes from stdin)");
}

fn print_human(path: &str, findings: &[Finding]) {
    if findings.is_empty() {
        println!("{}: no issues found", path);
        return;
    }
    for finding in findings {
        println!(
            "{}:{}: {}: {} [{}]",
            path,
            finding.line,
            finding.severity.as_str(),
            finding.message,
            finding.rule
        );
    }
    let errors = findings.iter().filter(|f| f.severity == Severity::Error).count();
    let warnings = findings.len() - errors;
    println!("{} error(s), {} warning(s)", errors, warnings);
}

fn print_json(path: &str, findings: &[Finding]) {
    let mut out = String::new();
    out.push('{');
    out.push_str(&format!("\"file\":{},", json_string(path)));
    out.push_str("\"findings\":[");
    for (i, finding) in findings.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('{');
        out.push_str(&format!("\"line\":{},", finding.line));
        out.push_str(&format!(
            "\"severity\":{},",
            json_string(finding.severity.as_str())
        ));
        out.push_str(&format!("\"rule\":{},", json_string(finding.rule)));
        out.push_str(&format!("\"message\":{}", json_string(&finding.message)));
        out.push('}');
    }
    out.push_str("]}");
    println!("{}", out);
}

fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
