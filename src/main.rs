mod lint;
mod parser;

use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

use lint::{check_routes, Finding, Severity};

const KNOWN_RULES: &[&str] = &[
    "shadowed-route",
    "duplicate-route",
    "missing-leading-slash",
    "trailing-slash",
    "empty-segment",
    "unparsable-line",
];

fn is_known_rule(name: &str) -> bool {
    KNOWN_RULES.contains(&name)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    let mut json_output = false;
    let mut path: Option<String> = None;
    let mut only_rules: Vec<String> = Vec::new();
    let mut ignore_rules: Vec<String> = Vec::new();

    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--json" => json_output = true,
            "-h" | "--help" => {
                print_usage();
                return ExitCode::SUCCESS;
            }
            "--rule" => match args.next() {
                Some(name) if is_known_rule(&name) => only_rules.push(name),
                Some(name) => {
                    eprintln!("routelint: unknown rule \"{}\"", name);
                    return ExitCode::FAILURE;
                }
                None => {
                    eprintln!("routelint: --rule requires a rule name");
                    print_usage();
                    return ExitCode::FAILURE;
                }
            },
            "--ignore" => match args.next() {
                Some(name) if is_known_rule(&name) => ignore_rules.push(name),
                Some(name) => {
                    eprintln!("routelint: unknown rule \"{}\"", name);
                    return ExitCode::FAILURE;
                }
                None => {
                    eprintln!("routelint: --ignore requires a rule name");
                    print_usage();
                    return ExitCode::FAILURE;
                }
            },
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
    let findings = filter_findings(findings, &only_rules, &ignore_rules);

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

// Ignoring a rule drops its findings before severity counts and the exit
// code are computed, so it doubles as a way to stop a specific rule from
// failing a CI run without silencing everything else.
fn filter_findings(
    findings: Vec<Finding>,
    only_rules: &[String],
    ignore_rules: &[String],
) -> Vec<Finding> {
    findings
        .into_iter()
        .filter(|f| only_rules.is_empty() || only_rules.iter().any(|r| r == f.rule))
        .filter(|f| !ignore_rules.iter().any(|r| r == f.rule))
        .collect()
}

fn print_usage() {
    eprintln!("usage: routelint <routes-file> [--json] [--rule NAME]... [--ignore NAME]...");
    eprintln!("       routelint - [--json]   (read routes from stdin)");
    eprintln!();
    eprintln!("  --rule NAME    only report findings for this rule (repeatable)");
    eprintln!("  --ignore NAME  suppress findings for this rule (repeatable)");
    eprintln!();
    eprintln!("known rules: {}", KNOWN_RULES.join(", "));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(rule: &'static str) -> Finding {
        Finding {
            line: 1,
            severity: Severity::Warning,
            rule,
            message: "msg".to_string(),
        }
    }

    #[test]
    fn keeps_everything_with_no_filters() {
        let findings = vec![finding("duplicate-route"), finding("trailing-slash")];
        let result = filter_findings(findings, &[], &[]);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn only_rules_keeps_just_the_named_rules() {
        let findings = vec![finding("duplicate-route"), finding("trailing-slash")];
        let only = vec!["trailing-slash".to_string()];
        let result = filter_findings(findings, &only, &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].rule, "trailing-slash");
    }

    #[test]
    fn ignore_removes_the_named_rule() {
        let findings = vec![finding("duplicate-route"), finding("trailing-slash")];
        let ignore = vec!["duplicate-route".to_string()];
        let result = filter_findings(findings, &[], &ignore);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].rule, "trailing-slash");
    }

    #[test]
    fn ignore_wins_over_a_conflicting_only_rule() {
        let findings = vec![finding("duplicate-route")];
        let only = vec!["duplicate-route".to_string()];
        let ignore = vec!["duplicate-route".to_string()];
        let result = filter_findings(findings, &only, &ignore);
        assert!(result.is_empty());
    }

    #[test]
    fn recognizes_known_rule_names() {
        assert!(is_known_rule("shadowed-route"));
        assert!(!is_known_rule("not-a-real-rule"));
    }
}
