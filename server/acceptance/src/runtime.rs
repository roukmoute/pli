use std::{env, path::Path};

use tokio::{
    net::TcpListener,
    sync::{mpsc, oneshot},
};

use crate::{
    Error, Example, Feature, IR_ENV, Step,
    generator::Entrypoint,
    steps::{Problem, World},
};

pub(crate) type Restart = oneshot::Sender<std::net::SocketAddr>;

async fn serve(
    mut listener: TcpListener,
    directory: &Path,
    mut restarts: mpsc::Receiver<Restart>,
) -> Result<(), Error> {
    loop {
        let reply = tokio::select! {
            result = webmail_server::serve_with_admin(listener, directory, crate::steps::admin_config()) => return result,
            Some(reply) = restarts.recv() => reply,
        };
        listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let _ = reply.send(listener.local_addr()?);
    }
}

async fn run_case<'a>(
    steps: impl Iterator<Item = &'a Step>,
    row: &Example,
    journal: Option<&Path>,
) -> Result<(), Problem> {
    let directory = tempfile::tempdir().map_err(Problem::infra)?;
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .map_err(Problem::infra)?;
    let mut world = World::new(listener.local_addr().map_err(Problem::infra)?)?;
    let (restart, restarts) = mpsc::channel(1);
    world.restart = Some(restart);
    world.recordings = journal.map(|log| (directory.path().to_owned(), log.to_owned()));
    let scenario = async {
        for step in steps {
            world.step(&step.text, row).await.map_err(|e| match e {
                Problem::Step(message) => Problem::Step(format!("{} — {message}", step.text)),
                other => other,
            })?;
        }
        Ok(())
    };
    tokio::select! {
        result = serve(listener, directory.path(), restarts) => {
            Err(Problem::infra(result.err().unwrap_or_else(|| "Le service s’est arrêté".into())))
        }
        result = scenario => result,
    }
}

pub async fn run(entry: &Path, journal: &Path) -> Result<bool, Error> {
    let entry = Entrypoint::load(entry)?;
    let ir = env::var_os(IR_ENV).map(Into::into).unwrap_or(entry.ir_path);
    let feature = Feature::load(&ir)?;
    if feature.cases() != entry.cases {
        return Err("Les scénarios et exemples de l’IR diffèrent du point d’entrée généré".into());
    }
    let empty = Example::new();
    let mut passed = true;
    for (index, row) in entry.cases {
        let scenario = &feature.scenarios[index];
        let name = format!("{} / {} / exemple {}", feature.name, scenario.name, row + 1);
        let steps = feature.background.iter().chain(&scenario.steps);
        match run_case(
            steps,
            scenario.examples.get(row).unwrap_or(&empty),
            Some(journal),
        )
        .await
        {
            Ok(()) => println!("OK   {name}"),
            Err(Problem::Step(message)) => {
                eprintln!("ÉCHEC   {name} — {message}");
                passed = false;
            }
            Err(Problem::Infrastructure(e)) => return Err(format!("{name} — {e}").into()),
        }
    }
    Ok(passed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_restart_serves_the_same_database_on_an_ephemeral_loopback_port() {
        let dir = tempfile::tempdir().unwrap();
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let (restart, restarts) = mpsc::channel(1);
        let address = listener.local_addr().unwrap();
        tokio::select! {
            _ = serve(listener, dir.path(), restarts) => panic!("Arrêt du service de recette"),
            () = async {
                let mut world = World::new(address).unwrap();
                world.step("je consulte la disponibilité du webmail", &Example::new()).await.unwrap();
                let db = rusqlite::Connection::open(dir.path().join("webmail.db")).unwrap();
                db.execute_batch("INSERT INTO server_profiles
                    (name,imap_host,imap_port,imap_tls,smtp_host,smtp_port,smtp_tls,domains,active,local_test)
                    VALUES ('Banc','127.0.0.1',3993,'implicit','127.0.0.1',3465,'implicit','[]',0,1)").unwrap();
                let (reply, ready) = oneshot::channel();
                restart.send(reply).await.unwrap();
                let address = ready.await.unwrap();
                assert!(address.ip().is_loopback());
                assert!(address.port() > 1023);
                let mut restarted = World::new(address).unwrap();
                restarted.step("je consulte les serveurs de messagerie", &Example::new()).await.unwrap();
                restarted.step("le nombre de serveurs configurés est <nombre>", &Example::from([("nombre".into(), "1".into())])).await.unwrap();
            } => {}
        }
    }

    fn steps(lines: &[&str]) -> Vec<Step> {
        lines
            .iter()
            .map(|text| Step {
                text: text.to_string(),
            })
            .collect()
    }

    #[tokio::test]
    async fn background_and_scenario_share_the_http_response_for_this_example_only() {
        let background = steps(&["je consulte la disponibilité du webmail"]);
        let assertions = steps(&[
            "le service répond avec le code <http>",
            "l’état annoncé est <etat_recu>",
        ]);
        let row = Example::from([
            ("http".into(), "200".into()),
            ("etat_recu".into(), "ok".into()),
        ]);
        run_case(background.iter().chain(&assertions), &row, None)
            .await
            .unwrap();
        let error = run_case(assertions.iter(), &row, None).await.unwrap_err();
        assert!(matches!(error, Problem::Step(msg) if msg.contains("Aucune réponse HTTP")));
    }

    #[tokio::test]
    async fn failures_name_the_step_and_explain_the_example_or_assertion() {
        for (text, reason) in [
            ("je lis mes messages", "Étape sans implémentation"),
            ("l’état annoncé est <absent>", "<absent>"),
            (
                "le service répond avec le code <invalide>",
                "Code HTTP invalide",
            ),
            ("le service répond avec le code <autre>", "201"),
            ("l’état annoncé est <etat>", "indisponible"),
            (
                "l’état annoncé est <deux> <champs>",
                "Étape sans implémentation",
            ),
        ] {
            let row = Example::from([
                ("invalide".into(), "abc".into()),
                ("autre".into(), "201".into()),
                ("etat".into(), "indisponible".into()),
            ]);
            let scenario = steps(&["je consulte la disponibilité du webmail", text]);
            let error = run_case(scenario.iter(), &row, None).await.unwrap_err();
            assert!(
                matches!(error, Problem::Step(msg) if msg.contains(text) && msg.contains(reason))
            );
        }
    }
}
