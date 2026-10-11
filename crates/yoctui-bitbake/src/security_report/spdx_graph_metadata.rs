// Resolve local SPDX 3 references without fetching external graph nodes.
fn resolve_spdx_graph_metadata(
    graph: &[Value],
    components: &mut [SpdxComponent],
    limitations: &mut Vec<String>,
) {
    let mut index = BTreeMap::new();
    let mut duplicates = BTreeSet::new();
    for value in graph {
        if let Some(id) = value
            .get("spdxId")
            .or_else(|| value.get("@id"))
            .and_then(Value::as_str)
            && index.insert(id, value).is_some()
        {
            duplicates.insert(id);
        }
    }
    for id in duplicates {
        index.remove(id);
        push_limitation(
            limitations,
            "Duplicate SPDX graph identifiers are not resolved".into(),
        );
    }
    let mut licenses = BTreeMap::<(&str, &str), BTreeSet<&str>>::new();
    let mut unresolved = BTreeSet::new();
    for edge in graph {
        if !matches!(
            edge.get("type").and_then(Value::as_str),
            Some("Relationship" | "LifecycleScopedRelationship")
        ) {
            continue;
        }
        let Some(kind @ ("hasConcludedLicense" | "hasDeclaredLicense")) =
            edge.get("relationshipType").and_then(Value::as_str)
        else {
            continue;
        };
        let Some(from) = edge.get("from").and_then(Value::as_str) else {
            continue;
        };
        let Some(targets) = edge.get("to").and_then(Value::as_array) else {
            unresolved.insert((from, kind));
            continue;
        };
        if targets.is_empty() {
            unresolved.insert((from, kind));
        }
        for target in targets {
            let expression = target
                .as_str()
                .and_then(|id| index.get(id))
                .filter(|node| {
                    node.get("type").and_then(Value::as_str)
                        == Some("simplelicensing_LicenseExpression")
                })
                .and_then(|node| node.get("simplelicensing_licenseExpression"))
                .and_then(Value::as_str)
                .filter(|text| {
                    !text.is_empty()
                        && text.len() <= MAX_SECURITY_TEXT_BYTES
                        && !text.chars().any(char::is_control)
                });
            if let Some(expression) = expression {
                licenses.entry((from, kind)).or_default().insert(expression);
            } else {
                unresolved.insert((from, kind));
            }
        }
    }
    for component in components {
        let mut selected = None;
        for kind in ["hasConcludedLicense", "hasDeclaredLicense"] {
            let key = (component.identity.as_str(), kind);
            if unresolved.contains(&key) {
                push_limitation(limitations, "SPDX 3 package license references missing or unsupported locally were not resolved".into());
                break;
            }
            if let Some(expressions) = licenses.get(&key) {
                if expressions.len() == 1 {
                    selected = expressions.first().map(|text| (*text).to_owned());
                } else {
                    push_limitation(
                        limitations,
                        "Ambiguous SPDX 3 package license expressions were not combined".into(),
                    );
                }
                break;
            }
        }
        if selected.is_some() {
            component.license = selected;
        }
        if let Some(package) = index.get(component.identity.as_str())
            && let Some(supplier) = package.get("suppliedBy")
        {
            let agent = supplier.as_object().or_else(|| {
                supplier
                    .as_str()
                    .and_then(|id| index.get(id))
                    .and_then(|value| value.as_object())
            });
            if let Some(agent) = agent.filter(|agent| {
                matches!(
                    agent.get("type").and_then(Value::as_str),
                    Some("Organization" | "Person")
                )
            }) {
                component.supplier = bounded_json_string(agent, &["name"], limitations);
            } else {
                push_limitation(limitations, "SPDX 3 package supplier references missing or unsupported locally were not resolved".into());
            }
        }
    }
}
