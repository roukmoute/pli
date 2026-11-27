use std::{env, io, path::Path, process::ExitCode};

use pli_acceptance::{runtime, worker};

async fn run(entry: &Path) -> Result<bool, pli_acceptance::Error> {
    let journal = tempfile::NamedTempFile::new()?;
    webmail_server::init_logging(std::sync::Mutex::new(journal.reopen()?));
    runtime::run(entry, journal.path()).await
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let result = match args.as_slice() {
        [mode] if mode == "--worker" => worker::serve(io::stdin().lock(), io::stdout().lock())
            .await
            .map(|()| true),
        [entry] => run(Path::new(entry)).await,
        _ => Err("Usage : acceptance-runner <point d’entrée> | --worker".into()),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("Erreur d’infrastructure APS : {e}");
            ExitCode::from(2)
        }
    }
}
