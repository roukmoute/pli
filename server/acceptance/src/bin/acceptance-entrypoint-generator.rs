use std::{env, path::Path, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let [ir, output] = args.as_slice() else {
        eprintln!("Usage : acceptance-entrypoint-generator <json-ir> <répertoire>");
        return ExitCode::from(2);
    };
    match pli_acceptance::generator::generate(Path::new(ir), Path::new(output)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
