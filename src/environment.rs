use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Serialize, Deserialize)]
struct StackOutputsEntry {
    #[serde(rename(deserialize = "OutputKey"))]
    output_key: String,
    #[serde(rename(deserialize = "OutputValue"))]
    output_value: String,
}

#[derive(Debug)]
pub struct StackOutputs {
    pub api_url: String,
    pub table_name: String,
}

impl Default for StackOutputs {
    fn default() -> Self {
        return Self {
            api_url: String::from(""),
            table_name: String::from(""),
        };
    }
}

pub fn get_stack_outputs() -> Result<StackOutputs> {
    let outputs_path = PathBuf::from("./outputs.json");
    let outputs_contents = fs::read_to_string(outputs_path)?;

    let outputs: Vec<StackOutputsEntry> = serde_json::from_str(&outputs_contents)?;

    let outputs = outputs
        .iter()
        .fold(StackOutputs::default(), |mut acc, current_entry| {
            if current_entry.output_key == "Table" {
                acc.table_name = current_entry.output_value.to_owned();
            }

            if current_entry.output_key == "API" {
                acc.api_url = current_entry.output_value.to_owned();
            }

            return acc;
        });

    return Ok(outputs);
}
