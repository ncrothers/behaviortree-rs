pub fn shallow() -> String {
    const SHALLOW: &str = r#"
        <root>
            <BehaviorTree ID="main">
                <StatusNode status="Failure" />
            </BehaviorTree>
        </root>
    "#;

    SHALLOW.to_string()
}

pub fn deep(depth: u32) -> String {
    let mut inner = String::new();

    for _ in 0..depth {
        inner.push_str("<Inverter>");
    }

    inner.push_str("<StatusNode status=\"Failure\" />");

    for _ in 0..depth {
        inner.push_str("</Inverter>");
    }

    format!(
        r#"
        <root>
            <BehaviorTree ID="main">
                {inner}
            </BehaviorTree>
        </root>
    "#
    )
}
