use super::*;
use crate::parser::{Page, Template};

fn template(slots: Vec<SlotDecl>) -> Template {
    Template {
        version: "0.1".to_string(),
        page: Page {
            width: 612.0,
            height: 792.0,
        },
        slots,
        draw: Vec::new(),
        fonts: Vec::new(),
        assets: Vec::new(),
    }
}

#[test]
fn accepts_matching_data() {
    let template = template(vec![
        SlotDecl {
            name: "product_name".to_string(),
            ty: SlotType::String,
            required: true,
        },
        SlotDecl {
            name: "buy".to_string(),
            ty: SlotType::Int,
            required: true,
        },
        SlotDecl {
            name: "sale_price".to_string(),
            ty: SlotType::Decimal,
            required: true,
        },
    ]);

    let data = serde_json::json!({
        "product_name": "Coffee",
        "buy": 2,
        "sale_price": 9.99
    });

    assert_eq!(validate_data(&template, &data), Ok(()));
}

#[test]
fn errors_on_missing_required_slot() {
    let template = template(vec![SlotDecl {
        name: "product_name".to_string(),
        ty: SlotType::String,
        required: true,
    }]);

    let data = serde_json::json!({});

    assert_eq!(
        validate_data(&template, &data),
        Err(vec![ValidationError::MissingRequiredSlot {
            name: "product_name".to_string(),
        }])
    );
}

#[test]
fn errors_on_wrong_type() {
    let template = template(vec![SlotDecl {
        name: "buy".to_string(),
        ty: SlotType::Int,
        required: true,
    }]);

    let data = serde_json::json!({
        "buy": "2"
    });

    assert_eq!(
        validate_data(&template, &data),
        Err(vec![ValidationError::WrongType {
            name: "buy".to_string(),
            expected: SlotType::Int,
            found: "string".to_string(),
        }])
    );
}

#[test]
fn allows_missing_optional_slot() {
    let template = template(vec![SlotDecl {
        name: "description".to_string(),
        ty: SlotType::String,
        required: false,
    }]);

    let data = serde_json::json!({});

    assert_eq!(validate_data(&template, &data), Ok(()));
}

#[test]
fn errors_when_data_is_not_object() {
    let template = template(vec![SlotDecl {
        name: "product_name".to_string(),
        ty: SlotType::String,
        required: true,
    }]);

    let data = serde_json::json!(["not", "an", "object"]);

    assert_eq!(
        validate_data(&template, &data),
        Err(vec![ValidationError::DataMustBeObject])
    );
}

#[test]
fn returns_multiple_errors_for_single_payload() {
    let template = template(vec![
        SlotDecl {
            name: "name".to_string(),
            ty: SlotType::String,
            required: true,
        },
        SlotDecl {
            name: "qty".to_string(),
            ty: SlotType::Int,
            required: true,
        },
        SlotDecl {
            name: "price".to_string(),
            ty: SlotType::Decimal,
            required: true,
        },
    ]);

    let data = serde_json::json!({
        "name": 12,
        "price": "9.99"
    });

    let err = validate_data(&template, &data).unwrap_err();

    assert_eq!(err.len(), 3);
    assert!(err.contains(&ValidationError::WrongType {
        name: "name".to_string(),
        expected: SlotType::String,
        found: "number".to_string(),
    }));
    assert!(err.contains(&ValidationError::MissingRequiredSlot {
        name: "qty".to_string(),
    }));
    assert!(err.contains(&ValidationError::WrongType {
        name: "price".to_string(),
        expected: SlotType::Decimal,
        found: "string".to_string(),
    }));
}
