pub mod generator;
pub mod runtime;
mod steps;
pub mod worker;

use std::{collections::BTreeMap, fs, path::Path};

use serde::Deserialize;
pub use webmail_server::Error;

const IR_ENV: &str = "PLI_ACCEPTANCE_IR";
const ENTRYPOINT: &str = "run";
const INTERPRETER: &str = "#!/usr/bin/env acceptance-runner\n";

type Example = BTreeMap<String, String>;
type Case = (usize, usize);

#[derive(Deserialize)]
struct Feature {
    name: String,
    #[serde(default)]
    background: Vec<Step>,
    scenarios: Vec<Scenario>,
}

#[derive(Deserialize)]
struct Scenario {
    name: String,
    #[serde(deserialize_with = "scenario_steps")]
    steps: Vec<Step>,
    examples: Vec<Example>,
}

#[derive(Deserialize)]
struct Step {
    text: String,
}

// Le parseur Go représente les étapes vides par null, Babashka par [].
fn scenario_steps<'de, D: serde::Deserializer<'de>>(input: D) -> Result<Vec<Step>, D::Error> {
    Option::<Vec<Step>>::deserialize(input).map(Option::unwrap_or_default)
}

impl Feature {
    fn load(path: &Path) -> Result<Self, Error> {
        let bytes = fs::read(path).map_err(|e| format!("{} : {e}", path.display()))?;
        Ok(serde_json::from_slice(&bytes).map_err(|e| format!("{} : {e}", path.display()))?)
    }

    fn cases(&self) -> Vec<Case> {
        let mut cases = Vec::new();
        for (index, scenario) in self.scenarios.iter().enumerate() {
            for row in 0..scenario.examples.len().max(1) {
                cases.push((index, row));
            }
        }
        cases
    }
}
