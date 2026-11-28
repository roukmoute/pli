//! Application distante des opérations enregistrées, dans leur ordre d’arrivée.

use std::{
    collections::BTreeMap,
    sync::{Arc, PoisonError},
    time::Duration,
};

use tokio::{sync::Notify, task::spawn_blocking, time::timeout};

use super::Lecteur;
use crate::{
    Error,
    db::Database,
    operations::{Depot, Lot, comptes_en_attente, conclure, prochain},
    sync::imap::Refus,
};

const REPRISE: Duration = Duration::from_secs(10);

impl Lecteur {
    /// Au démarrage, les opérations restées en attente reprennent d’elles-mêmes.
    pub(super) fn reprendre(&self) {
        comptes_en_attente(&self.context.db)
            .into_iter()
            .flatten()
            .for_each(|compte| self.appliquer(compte));
    }

    /// Réveille l’application des opérations en attente du compte.
    pub(crate) fn appliquer(&self, compte: i64) {
        self.files
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(compte)
            .or_insert_with(|| {
                let reveil = Arc::new(Notify::new());
                tokio::spawn(self.clone().boucle(compte, reveil.clone()));
                reveil
            })
            .notify_one();
    }

    async fn boucle(self, compte: i64, reveil: Arc<Notify>) {
        loop {
            reveil.notified().await;
            while self.tenter(compte).await.is_err() {
                // Messagerie injoignable : nouvel essai au prochain réveil ou après le délai.
                tracing::warn!(account_id = compte, "Opérations en attente à reprendre");
                let _ = timeout(REPRISE, reveil.notified()).await;
            }
        }
    }

    async fn tenter(&self, compte: i64) -> Result<(), Error> {
        let _ecriture = self.ecriture(compte).await;
        self.rattraper(compte).await
    }

    /// Applique toutes les opérations en attente. L’appelant détient l’écriture du compte.
    pub(crate) async fn rattraper(&self, compte: i64) -> Result<(), Error> {
        let db = &self.context.db;
        while let Some(lot) = sqlite(db, move |db| prochain(db, compte)).await? {
            let depot = self.deposer(compte, &lot).await?;
            sqlite(db, move |db| conclure(db, compte, &lot, &depot)).await?;
        }
        Ok(())
    }

    async fn deposer(&self, compte: i64, lot: &Lot) -> Result<Depot, Error> {
        // Dossier disparu ou régénéré : plus aucune de ces lettres n’est joignable.
        let Some(chemin) = &lot.chemin else {
            return Ok(Depot::Applique(BTreeMap::new()));
        };
        let reponse = self
            .marquer(
                compte,
                lot.generation,
                chemin,
                &lot.uids,
                &lot.drapeau,
                lot.valeur,
            )
            .await;
        match reponse {
            Ok(etats) => Ok(Depot::Applique(etats)),
            Err(refus) if refus.is::<Refus>() => Ok(Depot::Refuse),
            Err(panne) => Err(panne),
        }
    }
}

async fn sqlite<T: Send + 'static>(
    db: &Arc<Database>,
    action: impl FnOnce(&Database) -> Result<T, Error> + Send + 'static,
) -> Result<T, Error> {
    let db = db.clone();
    spawn_blocking(move || action(&db)).await?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        sync::Context,
        test_support::{BOITES_DE_TEST, TestApp},
    };

    #[tokio::test]
    async fn le_demarrage_reprend_les_comptes_dont_une_operation_attend() -> Result<(), Error> {
        let app = TestApp::new()?;
        app.db.writer()?.execute_batch(BOITES_DE_TEST)?;
        app.db.writer()?.execute_batch(
            "INSERT INTO operations(account_id,id,kind) VALUES (2,'a','lu'),(2,'b','lu');
             INSERT INTO operations(account_id,id,kind,state) VALUES (1,'c','lu','confirmee');",
        )?;
        let lecteur = Lecteur {
            context: Arc::new(Context {
                db: app.db.clone(),
                key: None,
                allow_test: Default::default(),
            }),
            ecritures: Default::default(),
            files: Default::default(),
        };
        lecteur.reprendre();
        let repris: Vec<i64> = lecteur.files.lock().unwrap().keys().copied().collect();
        assert_eq!(repris, [2]);
        Ok(())
    }
}
