// Route table format: one route per line, "METHOD /path -> handler".
// Blank lines and lines starting with # are ignored.

pub struct Route {
    pub line: usize,
    pub method: String,
    pub path: String,
    pub handler: String,
}

pub fn parse(input: &str) -> (Vec<Route>, Vec<usize>) {
    let mut routes = Vec::new();
    let mut unparsable = Vec::new();

    for (i, raw_line) in input.lines().enumerate() {
        let line_no = i + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        match parse_line(line) {
            Some((method, path, handler)) => routes.push(Route {
                line: line_no,
                method,
                path,
                handler,
            }),
            None => unparsable.push(line_no),
        }
    }

    (routes, unparsable)
}

fn parse_line(line: &str) -> Option<(String, String, String)> {
    let arrow_pos = line.find("->")?;
    let (left, right) = line.split_at(arrow_pos);
    let handler = right[2..].trim();
    if handler.is_empty() {
        return None;
    }

    let mut parts = left.split_whitespace();
    let method = parts.next()?;
    let path = parts.next()?;
    if parts.next().is_some() {
        return None; // extra tokens between the method and the arrow
    }

    Some((method.to_uppercase(), path.to_string(), handler.to_string()))
}
