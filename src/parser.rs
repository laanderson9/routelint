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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_valid_line() {
        let (routes, unparsable) = parse("GET /users -> list_users");
        assert!(unparsable.is_empty());
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].line, 1);
        assert_eq!(routes[0].method, "GET");
        assert_eq!(routes[0].path, "/users");
        assert_eq!(routes[0].handler, "list_users");
    }

    #[test]
    fn uppercases_the_method() {
        let (routes, _) = parse("get /users -> list_users");
        assert_eq!(routes[0].method, "GET");
    }

    #[test]
    fn trims_surrounding_whitespace() {
        let (routes, _) = parse("   GET   /users   ->   list_users   ");
        assert_eq!(routes[0].path, "/users");
        assert_eq!(routes[0].handler, "list_users");
    }

    #[test]
    fn ignores_blank_lines_and_comments() {
        let (routes, unparsable) = parse("\n# a comment\n   \nGET /users -> list_users\n");
        assert_eq!(routes.len(), 1);
        assert!(unparsable.is_empty());
        // the route should still be attributed to its real line number
        assert_eq!(routes[0].line, 4);
    }

    #[test]
    fn rejects_a_line_without_an_arrow() {
        let (routes, unparsable) = parse("GET /users list_users");
        assert!(routes.is_empty());
        assert_eq!(unparsable, vec![1]);
    }

    #[test]
    fn rejects_a_line_with_no_handler() {
        let (routes, unparsable) = parse("GET /users ->");
        assert!(routes.is_empty());
        assert_eq!(unparsable, vec![1]);
    }

    #[test]
    fn rejects_extra_tokens_before_the_arrow() {
        let (routes, unparsable) = parse("GET /users extra -> list_users");
        assert!(routes.is_empty());
        assert_eq!(unparsable, vec![1]);
    }

    #[test]
    fn rejects_a_missing_path() {
        let (routes, unparsable) = parse("GET -> list_users");
        assert!(routes.is_empty());
        assert_eq!(unparsable, vec![1]);
    }

    #[test]
    fn tracks_line_numbers_across_multiple_lines() {
        let (routes, unparsable) = parse("GET /a -> a\nbogus\nPOST /b -> b");
        assert_eq!(routes.len(), 2);
        assert_eq!(routes[0].line, 1);
        assert_eq!(routes[1].line, 3);
        assert_eq!(unparsable, vec![2]);
    }
}
