use serde::{Deserialize, Serialize};

use crate::CapabilityId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UtilityMenuKind {
    Devtool,
    Recipetool,
    BitBakeLayers,
    Pkgdata,
    Core,
    Advanced,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UtilityMenuEntry {
    pub kind: UtilityMenuKind,
    pub operation: String,
    pub capability: Option<CapabilityId>,
    pub typed_fields: Vec<String>,
    pub destructive: bool,
    pub network: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExpertArguments {
    pub input: String,
    pub argv: Vec<String>,
    pub validation_error: Option<String>,
}

impl ExpertArguments {
    pub fn parse(&mut self) -> Result<&[String], String> {
        match parse_words(&self.input) {
            Ok(argv) => {
                self.argv = argv;
                self.validation_error = None;
                Ok(&self.argv)
            }
            Err(error) => {
                self.validation_error = Some(error.clone());
                Err(error)
            }
        }
    }
}

pub fn utility_menu_catalog() -> Vec<UtilityMenuEntry> {
    vec![
        (
            UtilityMenuKind::Devtool,
            "status",
            Some(CapabilityId::DevtoolStatus),
            false,
            false,
        ),
        (
            UtilityMenuKind::Devtool,
            "modify",
            Some(CapabilityId::DevtoolModify),
            true,
            false,
        ),
        (
            UtilityMenuKind::Recipetool,
            "create",
            Some(CapabilityId::RecipetoolCreateOutfile),
            true,
            false,
        ),
        (
            UtilityMenuKind::Recipetool,
            "appendfile",
            Some(CapabilityId::RecipetoolAppendFile),
            true,
            false,
        ),
        (
            UtilityMenuKind::BitBakeLayers,
            "show-layers",
            Some(CapabilityId::BitBakeLayersShowLayers),
            false,
            false,
        ),
        (
            UtilityMenuKind::BitBakeLayers,
            "add-layer",
            None,
            true,
            false,
        ),
        (
            UtilityMenuKind::Pkgdata,
            "lookup-pkg",
            Some(CapabilityId::PkgDataLookupPackage),
            false,
            false,
        ),
        (
            UtilityMenuKind::Core,
            "bitbake-target",
            Some(CapabilityId::BitBakeBuild),
            true,
            false,
        ),
        (UtilityMenuKind::Advanced, "expert-argv", None, false, false),
    ]
    .into_iter()
    .map(
        |(kind, operation, capability, destructive, network)| UtilityMenuEntry {
            kind,
            operation: operation.into(),
            capability,
            typed_fields: Vec::new(),
            destructive,
            network,
        },
    )
    .collect()
}

fn parse_words(input: &str) -> Result<Vec<String>, String> {
    if input
        .chars()
        .any(|ch| ch == '\0' || (ch.is_control() && !ch.is_whitespace()))
    {
        return Err("arguments contain control bytes".into());
    }
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escape = false;
    for ch in input.chars() {
        if escape {
            current.push(ch);
            escape = false;
            continue;
        }
        if ch == '\\' {
            escape = true;
            continue;
        }
        if ch == '\'' || ch == '"' {
            if quote == Some(ch) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(ch);
            } else {
                current.push(ch);
            }
            continue;
        }
        if ch.is_whitespace() && quote.is_none() {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
        } else {
            current.push(ch);
        }
    }
    if escape || quote.is_some() {
        return Err("unterminated quote or escape".into());
    }
    if !current.is_empty() {
        out.push(current);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn utility_menu_catalog_exposes_typed_common_operations() {
        let catalog = utility_menu_catalog();
        assert!(catalog.iter().any(|entry| entry.operation == "status"));
        assert!(catalog.iter().any(|entry| entry.operation == "lookup-pkg"));
        assert!(catalog.iter().any(|entry| entry.destructive));
    }
    #[test]
    fn utility_menu_expert_form_parses_argv_and_retains_errors() {
        let mut form = ExpertArguments {
            input: "--name 'core image'".into(),
            ..Default::default()
        };
        assert_eq!(form.parse().unwrap(), ["--name", "core image"]);
        form.input = "'unterminated".into();
        assert!(form.parse().is_err());
        assert!(form.validation_error.is_some());
    }
}
