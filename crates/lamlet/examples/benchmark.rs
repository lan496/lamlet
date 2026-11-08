//! Development-only JSON-lines adapter for tools/compare_ast.py.
use lamlet::{Options, compile_with_options, source::Source, syntax};
use serde::Deserialize;
use serde_json::json;
use std::{
    hint::black_box,
    io::{self, BufRead, Write},
    time::Instant,
};

#[derive(Deserialize)]
struct Request {
    path: String,
    repeats: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let started = Instant::now();
    black_box(lamlet::manual_lint::catalog());
    let mut output = io::BufWriter::new(io::stdout().lock());
    writeln!(
        output,
        "{}",
        json!({"ready": true, "init_seconds": started.elapsed().as_secs_f64()})
    )?;
    output.flush()?;
    for line in io::stdin().lock().lines() {
        let request: Request = serde_json::from_str(&line?)?;
        let bytes = std::fs::read(&request.path)?;
        let mut parse = vec![];
        let mut check = vec![];
        let mut serialize = vec![];
        let mut report = String::new();
        for _ in 0..request.repeats {
            let started = Instant::now();
            let _ = black_box(syntax::parse(black_box(&bytes), false));
            parse.push(started.elapsed().as_secs_f64());
            let started = Instant::now();
            let source = Source::new(&request.path, bytes.clone());
            let compiled = compile_with_options(&source, &Options::default());
            check.push(started.elapsed().as_secs_f64());
            let started = Instant::now();
            report = serde_json::to_string(&compiled.report(&source))?;
            serialize.push(started.elapsed().as_secs_f64());
            black_box(&compiled.output);
        }
        writeln!(
            output,
            "{}",
            json!({"parse_seconds": parse, "check_seconds": check,
            "serialize_seconds": serialize, "report_json": report})
        )?;
        output.flush()?;
    }
    Ok(())
}
