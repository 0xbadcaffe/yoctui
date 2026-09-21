fn definition(id: CapabilityId) -> Definition {
    definition_core(id)
        .or_else(|| definition_raw(id))
        .or_else(|| definition_development(id))
        .or_else(|| definition_workflows(id))
        .expect("every capability has a built-in catalog definition")
}
