use colored::Colorize;
use std::borrow::Cow;
use std::env;
use std::ffi::OsStr;
use std::fmt::{self, Display};
use std::fs::File;
use std::io::Read;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use syn::visit::{self, Visit};
use syn::{ExprUnsafe, ItemFn, Stmt};

struct StmtVisitor {
    count: usize,
    unsafe_count: usize,
    in_unsafe: u32,
}

impl<'ast> Visit<'ast> for StmtVisitor {
    fn visit_expr_unsafe(&mut self, node: &'ast ExprUnsafe) {
        self.in_unsafe += 1;
        visit::visit_expr_unsafe(self, node);
        self.in_unsafe -= 1;
    }
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let unsafety = node.sig.unsafety.is_some();
        if unsafety {
            self.in_unsafe += 1;
        }
        visit::visit_item_fn(self, node);
        if unsafety {
            self.in_unsafe -= 1;
        }
    }
    fn visit_stmt(&mut self, node: &'ast Stmt) {
        self.count += 1;
        if self.in_unsafe > 0 {
            self.unsafe_count += 1;
        }
        visit::visit_stmt(self, node);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() <= 1 {
        println!("no input provided");
        return;
    }

    let mut total = 0;
    let mut unsafe_total = 0;
    for filename in &args[1..] {
        let mut src = String::new();
        let mut file = File::open(filename).expect("Unable to open source file");
        file.read_to_string(&mut src)
            .expect("Unable to read input file");

        let ast = match syn::parse_file(&src) {
            Err(error) => {
                let err = Error::ParseFile {
                    error,
                    filepath: PathBuf::from(filename),
                    source_code: src,
                };
                let _ = writeln!(io::stderr(), "{}", err);
                continue;
            }
            Ok(ast) => ast,
        };

        let mut visitor = StmtVisitor {
            count: 0,
            unsafe_count: 0,
            in_unsafe: 0,
        };
        visitor.visit_file(&ast);

        println!("{}: {}/{}", filename, visitor.unsafe_count, visitor.count);

        total += visitor.count;
        unsafe_total += visitor.unsafe_count;
    }

    println!("total: {}/{}", unsafe_total, total);
}

fn render_location(
    formatter: &mut fmt::Formatter,
    err: &syn::Error,
    filepath: &Path,
    code: &str,
) -> fmt::Result {
    let start = err.span().start();
    let mut end = err.span().end();

    if start.line == end.line && start.column == end.column {
        return render_fallback(formatter, err);
    }

    let code_line = match code.lines().nth(start.line - 1) {
        Some(line) => line,
        None => return render_fallback(formatter, err),
    };

    if end.line > start.line {
        end.line = start.line;
        end.column = code_line.len();
    }

    let filename = filepath
        .file_name()
        .map(OsStr::to_string_lossy)
        .unwrap_or(Cow::Borrowed("main.rs"));

    write!(
        formatter,
        "\n\
         {error}{header}\n\
         {indent}{arrow} {filename}:{linenum}:{colnum}\n\
         {indent} {pipe}\n\
         {label} {pipe} {code}\n\
         {indent} {pipe} {offset}{underline} {message}\n\
         ",
        error = "error".red().bold(),
        header = ": Syn unable to parse file".bold(),
        indent = " ".repeat(start.line.to_string().len()),
        arrow = "-->".blue().bold(),
        filename = filename,
        linenum = start.line,
        colnum = start.column,
        pipe = "|".blue().bold(),
        label = start.line.to_string().blue().bold(),
        code = code_line.trim_end(),
        offset = " ".repeat(start.column),
        underline = "^".repeat(end.column - start.column).red().bold(),
        message = err.to_string().red(),
    )
}

fn render_fallback(formatter: &mut fmt::Formatter, err: &syn::Error) -> fmt::Result {
    write!(formatter, "Unable to parse file: {}", err)
}

enum Error {
    ParseFile {
        error: syn::Error,
        filepath: PathBuf,
        source_code: String,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Error::*;

        match self {
            ParseFile {
                error,
                filepath,
                source_code,
            } => render_location(f, error, filepath, source_code),
        }
    }
}
