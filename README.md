# routelint

A linter for URL route tables. It catches the class of bug where a router
works fine in testing and then silently sends requests to the wrong handler
in production, because most routers (Express, Sinatra, Flask, and plenty of
hand-rolled ones) try routes in declaration order and dispatch to the first
match. If a dynamic route like `/users/:id` is declared before a static one
like `/users/new`, the static route is dead code: every request for
`/users/new` gets captured by `:id` first, and nothing tells you.

routelint reads a plain-text route table and reports:

- `shadowed-route` — a static route that a dynamic route declared earlier
  will always intercept
- `duplicate-route` — the same method and path defined twice
- `missing-leading-slash` — a path that doesn't start with `/`
- `trailing-slash` — a path other than `/` ending in `/`
- `empty-segment` — a path containing `//`
- `unparsable-line` — a line that isn't blank, a comment, or a valid route

## Route file format

One route per line:

```
METHOD /path -> handler_name
```

Blank lines and lines starting with `#` are ignored. See
`examples/sample.routes` for a file with several of the issues above.

## Usage

```
$ routelint examples/sample.routes
examples/sample.routes:4: warning: GET /users/new can never match: line 3 (/users/:id) captures it first [shadowed-route]
examples/sample.routes:6: error: GET /users/:id is already defined on line 3 [duplicate-route]
examples/sample.routes:7: warning: path "/health/" has a trailing slash [trailing-slash]
examples/sample.routes:8: error: path "//status" contains an empty segment [empty-segment]
examples/sample.routes:9: warning: line does not match "METHOD /path -> handler" [unparsable-line]
2 error(s), 3 warning(s)
```

Findings are sorted by line number, and the exit code is nonzero if any
finding is an error. That makes it usable as a CI check on its own.

### JSON output

Pass `--json` to get machine-readable output instead, for feeding into
editors or other tooling:

```
$ routelint examples/sample.routes --json
{"file":"examples/sample.routes","findings":[{"line":4,"severity":"warning","rule":"shadowed-route","message":"..."}, ...]}
```

Each finding is an object with `line`, `severity` (`"error"` or
`"warning"`), `rule`, and `message`. The top-level object also has `file`.

## Building

```
cargo build --release
```

No third-party dependencies — the whole thing is standard library.

## Status

Early. The route file format is a stand-in until there's parsing for real
framework route definitions (Express `app.get(...)` calls, Rust `axum`
routers, and so on). See the issues for what's planned.
