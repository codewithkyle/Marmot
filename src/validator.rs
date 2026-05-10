use crate::parser::{SlotDecl, SlotType, Template};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    DataMustBeObject,
    MissingRequiredSlot {
        name: String,
    },
    WrongType {
        name: String,
        expected: SlotType,
        found: String,
    },
}

pub fn validate_data(template: &Template, data: &Value) -> Result<(), Vec<ValidationError>> {
    let Some(object) = data.as_object() else {
        return Err(vec![ValidationError::DataMustBeObject]);
    };

    let mut errors = Vec::new();

    for slot in &template.slots {
        validate_slot(slot, object.get(&slot.name), &mut errors);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_slot(slot: &SlotDecl, value: Option<&Value>, errors: &mut Vec<ValidationError>) {
    let Some(value) = value else {
        if slot.required {
            errors.push(ValidationError::MissingRequiredSlot {
                name: slot.name.clone(),
            });
        }
        return;
    };

    if !value_matches_slot_type(value, &slot.ty) {
        errors.push(ValidationError::WrongType {
            name: slot.name.clone(),
            expected: slot.ty.clone(),
            found: json_type_name(value).to_string(),
        });
    }
}

fn value_matches_slot_type(value: &Value, ty: &SlotType) -> bool {
    match ty {
        SlotType::String => value.is_string(),
        SlotType::Int => value.as_i64().is_some() || value.as_u64().is_some(),
        SlotType::Decimal => value.is_number(),
    }
}

fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
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
}
