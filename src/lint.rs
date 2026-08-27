use crate::parser::Route;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub line: usize,
    pub severity: Severity,
    pub rule: &'static str,
    pub message: String,
}

pub fn check_routes(routes: &[Route]) -> Vec<Finding> {
    let mut findings = Vec::new();

    check_path_shape(routes, &mut findings);
    check_duplicates(routes, &mut findings);
    check_shadowing(routes, &mut findings);

    findings.sort_by_key(|f| f.line);
    findings
}

fn check_path_shape(routes: &[Route], findings: &mut Vec<Finding>) {
    for route in routes {
        if !route.path.starts_with('/') {
            findings.push(Finding {
                line: route.line,
                severity: Severity::Error,
                rule: "missing-leading-slash",
                message: format!("path \"{}\" does not start with /", route.path),
            });
        }
        if route.path.len() > 1 && route.path.ends_with('/') {
            findings.push(Finding {
                line: route.line,
                severity: Severity::Warning,
                rule: "trailing-slash",
                message: format!("path \"{}\" has a trailing slash", route.path),
            });
        }
        if route.path.contains("//") {
            findings.push(Finding {
                line: route.line,
                severity: Severity::Error,
                rule: "empty-segment",
                message: format!("path \"{}\" contains an empty segment", route.path),
            });
        }
    }
}

fn check_duplicates(routes: &[Route], findings: &mut Vec<Finding>) {
    for (i, route) in routes.iter().enumerate() {
        for earlier in &routes[..i] {
            if earlier.method == route.method && earlier.path == route.path {
                findings.push(Finding {
                    line: route.line,
                    severity: Severity::Error,
                    rule: "duplicate-route",
                    message: format!(
                        "{} {} is already defined on line {}",
                        route.method, route.path, earlier.line
                    ),
                });
                break;
            }
        }
    }
}

fn check_shadowing(routes: &[Route], findings: &mut Vec<Finding>) {
    for (i, route) in routes.iter().enumerate() {
        for earlier in &routes[..i] {
            if earlier.method != route.method {
                continue;
            }
            if earlier.path == route.path {
                continue; // already reported by check_duplicates
            }
            if segments_shadow(&earlier.path, &route.path) {
                findings.push(Finding {
                    line: route.line,
                    severity: Severity::Warning,
                    rule: "shadowed-route",
                    message: format!(
                        "{} {} can never match: line {} ({}) captures it first",
                        route.method, route.path, earlier.line, earlier.path
                    ),
                });
            }
        }
    }
}

// Most routers try routes in declaration order and use the first match,
// so a dynamic segment (":id") declared before a static one ("new") at the
// same position silently swallows every request meant for the static route.
fn segments_shadow(earlier_path: &str, later_path: &str) -> bool {
    let earlier_segs: Vec<&str> = earlier_path.split('/').filter(|s| !s.is_empty()).collect();
    let later_segs: Vec<&str> = later_path.split('/').filter(|s| !s.is_empty()).collect();

    if earlier_segs.len() != later_segs.len() || earlier_segs.is_empty() {
        return false;
    }

    let mut has_param = false;
    for (earlier_seg, later_seg) in earlier_segs.iter().zip(later_segs.iter()) {
        if earlier_seg.starts_with(':') {
            has_param = true;
            continue;
        }
        if earlier_seg != later_seg {
            return false;
        }
    }
    has_param
}
